// search-core/src/rpc_compat.rs
//! RPC 适配层 - 将 rpc crate 的类型转换为内部类型

use rpc::search::{SearchRequest as RpcSearchRequest, SearchResult as RpcSearchResult, SortMode};
use crate::{SearchEngine, SearchHit};
use std::path::PathBuf;

/// 从 RPC SearchRequest 执行搜索
pub fn handle_search_request(
    _engine: &SearchEngine,
    req: &RpcSearchRequest,
) -> RpcSearchResult {
    // 验证请求
    if req.root_directories.is_empty() {
        return RpcSearchResult::Failed("root_directories 不能为空".to_string());
    }
    
    // 构建查询字符串
    let query = build_query_string(req);
    
    if query.is_empty() {
        return RpcSearchResult::Failed("没有有效的搜索条件".to_string());
    }
    
    // TODO: 实际实现应该是异步的，返回 session_id 并通过流返回结果
    // 这里先返回启动成功
    RpcSearchResult::Started { session_id: 1, total_count: 0 }
}

/// 构建查询字符串
fn build_query_string(req: &RpcSearchRequest) -> String {
    tracing::debug!("[请求解析] keywords: {:?}, semantic_queries: {:?}", 
                    req.keywords, req.semantic_queries);
    
    let mut parts = Vec::new();
    
    // 关键词搜索
    for keyword in &req.keywords {
        parts.push(keyword.clone());
    }
    
    // 语义查询（交给 AI 处理）
    for semantic in &req.semantic_queries {
        parts.push(semantic.clone());
    }
    
    let query = parts.join(" ");
    tracing::debug!("[请求解析] 合并后的查询字符串: '{}'", query);
    query
}

/// 从 RPC SortMode 转换
pub fn convert_sort_mode(mode: &SortMode) -> crate::search::SortMode {
    match mode {
        SortMode::Alphabetical => crate::search::SortMode::Alphabetical,
        SortMode::ReverseAlphabetical => crate::search::SortMode::ReverseAlphabetical,
        SortMode::AccessedTime => crate::search::SortMode::AccessedTime,
        SortMode::CreatedTime => crate::search::SortMode::CreatedTime,
        SortMode::ModifiedTime => crate::search::SortMode::ModifiedTime,
        SortMode::Extension => crate::search::SortMode::Extension,
        SortMode::Relevance => crate::search::SortMode::Relevance,
    }
}

/// 搜索结果项（用于流式返回）
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    pub path: PathBuf,
    pub title: String,
    pub score: f32,
    pub preview: String,
    pub ags: Vec<String>,
    pub file_size: u64,
    pub modified_time: std::time::SystemTime,
}

impl From<SearchHit> for SearchResultItem {
    fn from(hit: SearchHit) -> Self {
        Self {
            path: PathBuf::from(&hit.path),
            title: hit.title.clone(),
            score: hit.score,
            preview: hit.title.clone(), // SearchHit 没有 preview，使用 title
            tags: hit.tags.map(|t| t.split_whitespace().map(String::from).collect()).unwrap_or_default(),
            file_size: 0, // TODO: 从索引获取
            modified_time: std::time::SystemTime::now(), // TODO: 从索引获取
        }
    }
}

/// 执行同步搜索（用于简单场景）
pub fn search_sync(
    engine: &SearchEngine,
    req: &RpcSearchRequest,
) -> Result<Vec<SearchResultItem>, String> {
    let query = build_query_string(req);
    
    if query.is_empty() {
        return Err("没有有效的搜索条件".to_string());
    }
    
    // 使用 AI 优化查询
    let refined_query = if !req.semantic_queries.is_empty() {
        tracing::info!("[搜索] 使用语义查询模式，原始查询: '{}'", query);
        let refined = engine.refine_query(&query);
        tracing::info!("[搜索] AI 提取的关键词: '{}'", refined);
        refined
    } else {
        tracing::info!("[搜索] 使用传统查询模式: '{}'", query);
        query.clone()
    };
    
    // 执行搜索
    // 如果有语义查询，使用混合搜索，否则使用传统搜索
    let results = if !req.semantic_queries.is_empty() {
        tracing::info!("[搜索] 使用混合搜索模式");
        // 混合搜索：结合传统全文搜索和简化的语义匹配
        let use_semantic = true;
        let text_weight = 0.5;  // 传统搜索权重
        let semantic_weight = 0.5;  // 语义搜索权重
        let limit = req.max_results.unwrap_or(100);
        
        engine.hybrid_search(&refined_query, use_semantic, text_weight, semantic_weight, limit)
            .map_err(|e| e.to_string())?
    } else {
        tracing::info!("[搜索] 使用纯传统搜索模式");
        // 纯传统搜索
        engine.search(&refined_query)
            .map_err(|e| e.to_string())?
    };
    
    tracing::info!("[搜索] 执行完成，原始结果数: {}", results.len());
    
    // 应用过滤器
    let mut filtered: Vec<SearchResultItem> = results
        .into_iter()
        .map(SearchResultItem::from)
        .collect();
    
    // 应用 root_directories 过滤（关键！只返回这些目录下的文件）
    if !req.root_directories.is_empty() {
        filtered.retain(|item| {
            req.root_directories.iter().any(|root| {
                item.path.starts_with(root)
            })
        });
    }
    
    // 应用 include/exclude globs
    if !req.include_globs.is_empty() {
        filtered.retain(|item| {
            req.include_globs.iter().any(|glob| {
                glob::Pattern::new(glob)
                    .map(|p| p.matches(item.path.to_str().unwrap_or("")))
                    .unwrap_or(false)
            })
        });
    }
    
    if !req.exclude_globs.is_empty() {
        filtered.retain(|item| {
            !req.exclude_globs.iter().any(|glob| {
                glob::Pattern::new(glob)
                    .map(|p| p.matches(item.path.to_str().unwrap_or("")))
                    .unwrap_or(false)
            })
        });
    }
    
    // 应用语义阈值
    if let Some(threshold) = req.semantic_threshold {
        filtered.retain(|item| item.score >= threshold);
    }
    
    // 限制结果数量
    if let Some(max) = req.max_results {
        filtered.truncate(max);
    }
    
    Ok(filtered)
}
