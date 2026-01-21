// search-core/src/search.rs
//! 搜索模块

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{Index, IndexReader, TantivyDocument};
use tantivy::schema::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::SearchHit;
use crate::schema::{FIELD_TITLE, FIELD_BODY, FIELD_PATH, FIELD_TAGS};

/// 排序模式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum SortMode {
    #[default]
    Relevance,
    Alphabetical,
    ReverseAlphabetical,
    AccessedTime,
    CreatedTime,
    ModifiedTime,
    Extension,
}

/// 搜索索引（打印结果版本，用于 CLI）
pub fn search_index(reader: &IndexReader, index: &Index, query_str: &str) -> Result<()> {
    let results = search_with_results(reader, index, query_str)?;
    
    if results.is_empty() {
        println!("     没有找到相关文档");
    }

    for hit in results {
        println!("   [{}] (Score: {:.2}) \n       路径: {}", hit.title, hit.score, hit.path);
    }

    Ok(())
}

/// 搜索索引（返回结果版本，用于 API）
pub fn search_with_results(reader: &IndexReader, index: &Index, query_str: &str) -> Result<Vec<SearchHit>> {
    tracing::debug!("[Tantivy 搜索] 查询字符串: '{}'", query_str);
    
    let searcher = reader.searcher();
    
    let schema = index.schema();
    let title_field = schema.get_field(FIELD_TITLE).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let tags_field = schema.get_field(FIELD_TAGS).ok();
    
    // 获取时间和大小字段
    let file_size_field = schema.get_field(crate::schema::FIELD_FILE_SIZE).ok();
    let modified_time_field = schema.get_field(crate::schema::FIELD_MODIFIED_TIME).ok();
    let created_time_field = schema.get_field(crate::schema::FIELD_CREATED_TIME).ok();
    let accessed_time_field = schema.get_field(crate::schema::FIELD_ACCESSED_TIME).ok();

    let query_parser = QueryParser::for_index(index, vec![title_field, body_field]);
    
    let query = match query_parser.parse_query(query_str) {
        Ok(q) => {
            tracing::debug!("[Tantivy 搜索] 解析后的查询: {:?}", q);
            q
        },
        Err(e) => {
            tracing::warn!("[Tantivy 搜索] 查询语法错误: '{}' - {}", query_str, e);
            return Ok(vec![]);
        }
    };

    let top_docs = searcher.search(&query, &TopDocs::with_limit(20))?;
    tracing::debug!("[Tantivy 搜索] 找到 {} 个文档", top_docs.len());
    
    let mut results = Vec::new();
    for (score, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

        let title = retrieved_doc.get_first(title_field)
            .and_then(|v| v.as_str())
            .unwrap_or("无标题")
            .to_string();
        
        let path = retrieved_doc.get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("无路径")
            .to_string();
        
        let tags = tags_field.and_then(|f| {
            retrieved_doc.get_first(f).and_then(|v| v.as_str()).map(|s| s.to_string())
        });
        
        // 从索引中读取时间和大小字段
        let file_size = file_size_field
            .and_then(|f| retrieved_doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let modified_time = modified_time_field
            .and_then(|f| retrieved_doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let created_time = created_time_field
            .and_then(|f| retrieved_doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let accessed_time = accessed_time_field
            .and_then(|f| retrieved_doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        results.push(SearchHit {
            title,
            path,
            score,
            tags,
            file_size,
            modified_time,
            created_time,
            accessed_time,
        });
    }

    Ok(results)
}

/// 搜索结果（带分页）
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

/// 带分页的搜索
pub fn search_with_pagination(
    reader: &IndexReader, 
    index: &Index, 
    query_str: &str,
    offset: usize,
    limit: usize,
) -> Result<SearchResults> {
    let all_results = search_with_results(reader, index, query_str)?;
    let total = all_results.len();
    
    let hits: Vec<SearchHit> = all_results
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    Ok(SearchResults {
        hits,
        total,
        offset,
        limit,
    })
}

/// 混合搜索：结合传统全文搜索和语义向量搜索
/// 
/// # 搜索策略
/// 1. 传统搜索：使用 Tantivy QueryParser 进行精确关键词匹配
/// 2. 语义搜索：使用 BERT embeddings 计算向量相似度（需要提供查询向量）
/// 3. 结果融合：使用加权平均合并两种搜索的分数
/// 
/// # 注意
/// 由于完整的向量相似度搜索需要遍历所有文档并计算相似度，在大规模数据集上性能较差。
/// 实际生产环境应该使用专门的向量数据库（如 Qdrant、Milvus）或 Tantivy 的自定义评分器。
pub fn hybrid_search(
    reader: &IndexReader,
    index: &Index,
    query_str: &str,
    query_embedding: Option<&[f32]>,  // 查询的向量表示
    text_weight: f32,   // 传统搜索权重（0.0-1.0）
    semantic_weight: f32, // 语义搜索权重（0.0-1.0）
    limit: usize,
) -> Result<Vec<SearchHit>> {
    use std::collections::HashMap;
    
    // 1. 传统全文搜索
    let text_results = search_with_results(reader, index, query_str)?;
    
    // 如果没有提供查询向量或语义权重为0，只返回传统搜索结果
    if query_embedding.is_none() || semantic_weight == 0.0 {
        let mut results = text_results;
        results.truncate(limit);
        return Ok(results);
    }
    
    // 注意：这里虽然有查询向量，但当前简化实现并未使用
    // 完整实现应该：计算文档向量并与查询向量做余弦相似度
    // let _query_vec = query_embedding.unwrap();
    
    // 2. 语义向量搜索
    // 注意：这是一个简化实现，实际应该：
    // - 预先计算并存储所有文档的向量
    // - 使用向量数据库或近似最近邻算法（ANN）加速搜索
    // - 或者使用 Tantivy 的自定义评分器
    
    let searcher = reader.searcher();
    let schema = index.schema();
    let title_field = schema.get_field(FIELD_TITLE).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let tags_field = schema.get_field(FIELD_TAGS).ok();
    let file_size_field = schema.get_field(crate::schema::FIELD_FILE_SIZE).ok();
    let modified_time_field = schema.get_field(crate::schema::FIELD_MODIFIED_TIME).ok();
    let created_time_field = schema.get_field(crate::schema::FIELD_CREATED_TIME).ok();
    let accessed_time_field = schema.get_field(crate::schema::FIELD_ACCESSED_TIME).ok();
    
    let mut semantic_results: Vec<SearchHit> = Vec::new();
    
    // 这里使用简化的语义匹配：基于标签和关键词的软匹配
    // 实际应该计算文档向量和查询向量的余弦相似度
    for segment_reader in searcher.segment_readers() {
        let store_reader = segment_reader.get_store_reader(1)?;
        for doc_id in 0..segment_reader.num_docs() {
            if let Ok(doc) = store_reader.get::<TantivyDocument>(doc_id) {
                let title = doc.get_first(title_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let path = doc.get_first(path_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let body = doc.get_first(body_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                let tags = tags_field.and_then(|f| {
                    doc.get_first(f).and_then(|v| v.as_str()).map(String::from)
                });
                
                // 从索引中读取时间和大小字段
                let file_size = file_size_field
                    .and_then(|f| doc.get_first(f))
                    .and_then(|v| v.as_u64());
                
                let modified_time = modified_time_field
                    .and_then(|f| doc.get_first(f))
                    .and_then(|v| v.as_u64());
                
                let created_time = created_time_field
                    .and_then(|f| doc.get_first(f))
                    .and_then(|v| v.as_u64());
                
                let accessed_time = accessed_time_field
                    .and_then(|f| doc.get_first(f))
                    .and_then(|v| v.as_u64());
                
                // 简化的语义相似度：基于关键词覆盖率
                let mut score = 0.0f32;
                let query_terms: Vec<&str> = query_str.split_whitespace().collect();
                
                for term in &query_terms {
                    if title.to_lowercase().contains(&term.to_lowercase()) {
                        score += 0.5;
                    }
                    if body.to_lowercase().contains(&term.to_lowercase()) {
                        score += 0.3;
                    }
                    if let Some(ref t) = tags {
                        if t.to_lowercase().contains(&term.to_lowercase()) {
                            score += 0.7; // 标签匹配权重更高
                        }
                    }
                }
                
                if score > 0.0 {
                    semantic_results.push(SearchHit {
                        title,
                        path,
                        score,
                        tags,
                        file_size,
                        modified_time,
                        created_time,
                        accessed_time,
                    });
                }
            }
        }
    }
    
    // 3. 融合两种搜索结果
    let mut combined_results: HashMap<String, SearchHit> = HashMap::new();
    
    // 标准化传统搜索分数
    let max_text_score = text_results.iter()
        .map(|h| h.score)
        .fold(0.0f32, f32::max)
        .max(1.0);
    
    for mut hit in text_results {
        hit.score = (hit.score / max_text_score) * text_weight;
        combined_results.insert(hit.path.clone(), hit);
    }
    
    // 标准化语义搜索分数
    let max_semantic_score = semantic_results.iter()
        .map(|h| h.score)
        .fold(0.0f32, f32::max)
        .max(1.0);
    
    for mut hit in semantic_results {
        let normalized_score = (hit.score / max_semantic_score) * semantic_weight;
        
        combined_results.entry(hit.path.clone())
            .and_modify(|existing| {
                // 已存在：合并分数
                existing.score += normalized_score;
            })
            .or_insert_with(|| {
                // 新结果
                hit.score = normalized_score;
                hit
            });
    }
    
    // 4. 按分数排序
    let mut results: Vec<SearchHit> = combined_results.into_values().collect();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    
    Ok(results)
}
