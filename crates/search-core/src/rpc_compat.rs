// search-core/src/rpc_compat.rs
//! RPC 适配层 - 将 rpc crate 的类型转换为内部类型
//! 
//! 新版本使用统一的 Query DSL 语法，例如：
//! - `foo AND bar` - 同时包含 foo 和 bar
//! - `*.rs size:>1MB` - Rust 文件且大于 1MB  
//! - `root:/home/dev AND mtime:<1w` - 指定目录下最近一周修改的文件

use rpc::search::{SearchRequest as RpcSearchRequest, SearchMode};
use crate::{SearchEngine, SearchHit};
use crate::query_executor::{parse_and_execute, QueryExecuteError};
use std::path::PathBuf;

/// 搜索结果项（用于流式返回）
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    pub path: PathBuf,
    pub title: String,
    pub score: f32,
    pub preview: String,
    pub tags: Vec<String>,
    pub file_size: u64,
    pub modified_time: std::time::SystemTime,
}

impl From<SearchHit> for SearchResultItem {
    fn from(hit: SearchHit) -> Self {
        // 将 Unix 时间戳转换为 SystemTime
        let modified_time = hit.modified_time
            .map(|secs| std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap_or_else(|| std::time::SystemTime::now());
        
        Self {
            path: PathBuf::from(&hit.path),
            title: hit.title.clone(),
            score: hit.score,
            preview: hit.title.clone(), // SearchHit 没有 preview，使用 title
            tags: hit.tags.map(|t| t.split_whitespace().map(String::from).collect()).unwrap_or_default(),
            file_size: hit.file_size.unwrap_or(0),
            modified_time,
        }
    }
}

/// 从 RPC SearchRequest 执行搜索
/// 
/// 根据 search_mode 决定搜索策略：
/// - Rule: 使用 Query DSL 解析器，支持精确匹配、正则、字段过滤等
/// - Natural: 使用 AI 语义搜索（混合传统搜索和向量搜索）
pub fn handle_search(
    engine: &SearchEngine,
    req: &RpcSearchRequest,
    limit: usize,
) -> Result<Vec<SearchResultItem>, QuerySearchError> {
    let query_str = &req.query;
    
    if query_str.is_empty() {
        return Err(QuerySearchError::ParseError("查询字符串不能为空".to_string()));
    }
    
    match req.search_mode {
        SearchMode::Rule => {
            // 规则搜索：使用 Query DSL 解析器
            tracing::info!("[搜索] Rule 模式，查询: '{}'", query_str);
            search_with_query_dsl(engine, query_str, limit)
        }
        SearchMode::Natural => {
            // 自然语言搜索：使用 AI 语义搜索
            tracing::info!("[搜索] Natural 模式，查询: '{}'", query_str);
            search_with_semantic(engine, query_str, limit)
        }
    }
}

/// 使用 Query DSL 执行规则搜索
/// 
/// 支持的语法：
/// - 布尔运算：`AND`, `OR`, `NOT`
/// - 字段过滤：`root:`, `size:`, `mtime:`, `glob:`, `regex:` 等
/// - 括号分组：`(foo AND bar) OR baz`
pub fn search_with_query_dsl(
    engine: &SearchEngine,
    query_str: &str,
    limit: usize,
) -> Result<Vec<SearchResultItem>, QuerySearchError> {
    tracing::info!("[Query DSL] 执行查询: '{}'", query_str);
    
    // 使用 Query 执行器解析并执行查询
    let results = parse_and_execute(&engine.reader, &engine.index, query_str, limit)
        .map_err(QuerySearchError::from)?;
    
    tracing::info!("[Query DSL] 找到 {} 个结果", results.len());
    
    // 转换为 RPC 兼容的结果类型
    let items: Vec<SearchResultItem> = results
        .into_iter()
        .map(SearchResultItem::from)
        .collect();
    
    Ok(items)
}

/// 使用 AI 语义搜索
/// 
/// 使用 BERT 模型提取关键词，结合传统全文搜索和向量相似度
pub fn search_with_semantic(
    engine: &SearchEngine,
    query_str: &str,
    limit: usize,
) -> Result<Vec<SearchResultItem>, QuerySearchError> {
    tracing::info!("[语义搜索] 执行查询: '{}'", query_str);
    
    // 使用 AI 优化查询
    let refined_query = engine.refine_query(query_str);
    tracing::info!("[语义搜索] AI 提取的关键词: '{}'", refined_query);
    
    // 混合搜索：结合传统全文搜索和语义匹配
    let results = engine.hybrid_search(
        &refined_query,
        true,   // use_semantic
        0.5,    // text_weight
        0.5,    // semantic_weight
        limit,
    ).map_err(|e| QuerySearchError::ExecutionError(e.to_string()))?;
    
    tracing::info!("[语义搜索] 找到 {} 个结果", results.len());
    
    let items: Vec<SearchResultItem> = results
        .into_iter()
        .map(SearchResultItem::from)
        .collect();
    
    Ok(items)
}

/// 智能搜索：根据查询内容自动选择搜索模式
/// 
/// 判断规则：
/// - 如果查询包含 Query DSL 语法（如 `AND`, `OR`, `NOT`, `field:`），使用 DSL 模式
/// - 否则使用传统全文搜索
pub fn smart_search(
    engine: &SearchEngine,
    query_str: &str,
    limit: usize,
) -> Result<Vec<SearchResultItem>, QuerySearchError> {
    // 检测是否是 Query DSL 语法
    let is_dsl = is_query_dsl_syntax(query_str);
    
    if is_dsl {
        tracing::info!("[智能搜索] 检测到 DSL 语法，使用 Query DSL 模式");
        search_with_query_dsl(engine, query_str, limit)
    } else {
        tracing::info!("[智能搜索] 使用传统全文搜索模式");
        // 使用传统搜索
        let results = engine.search(query_str)
            .map_err(|e| QuerySearchError::ExecutionError(e.to_string()))?;
        
        let items: Vec<SearchResultItem> = results
            .into_iter()
            .take(limit)
            .map(SearchResultItem::from)
            .collect();
        
        Ok(items)
    }
}

/// 检测查询字符串是否包含 Query DSL 语法
fn is_query_dsl_syntax(query: &str) -> bool {
    // 检查布尔操作符
    let has_boolean = query.contains(" AND ") 
        || query.contains(" OR ") 
        || query.starts_with("NOT ")
        || query.contains(" NOT ");
    
    // 检查字段语法（field:value）
    let has_field = query.contains("root:")
        || query.contains("path:")
        || query.contains("size:")
        || query.contains("mtime:")
        || query.contains("atime:")
        || query.contains("ctime:")
        || query.contains("glob:")
        || query.contains("name:")
        || query.contains("regex:")
        || query.contains("re:");
    
    has_boolean || has_field
}

/// Query 搜索错误
#[derive(Debug)]
pub enum QuerySearchError {
    /// 查询解析错误
    ParseError(String),
    /// 查询验证错误
    ValidationError(String),
    /// 执行错误
    ExecutionError(String),
}

impl From<QueryExecuteError> for QuerySearchError {
    fn from(err: QueryExecuteError) -> Self {
        match err {
            QueryExecuteError::ParseError(msg) => QuerySearchError::ParseError(msg),
            QueryExecuteError::ValidationError(e) => QuerySearchError::ValidationError(e.to_string()),
            QueryExecuteError::ExecutionError(msg) => QuerySearchError::ExecutionError(msg),
        }
    }
}

impl std::fmt::Display for QuerySearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuerySearchError::ParseError(msg) => write!(f, "查询解析错误: {}", msg),
            QuerySearchError::ValidationError(msg) => write!(f, "查询验证错误: {}", msg),
            QuerySearchError::ExecutionError(msg) => write!(f, "搜索执行错误: {}", msg),
        }
    }
}

impl std::error::Error for QuerySearchError {}
