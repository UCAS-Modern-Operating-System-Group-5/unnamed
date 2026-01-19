// search-core/src/query_executor.rs
//! Query DSL 执行器
//!
//! 将解析后的 Query AST 转换为实际的搜索操作。
//! 支持布尔逻辑（AND/OR/NOT）和各种过滤条件。

use std::path::Path;

use anyhow::Result;
use query::{Query, Term, ValidationError};
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, QueryParser};
use tantivy::schema::Value;
use tantivy::{Index, IndexReader, TantivyDocument};

use crate::schema::{FIELD_BODY, FIELD_FILE_SIZE, FIELD_MODIFIED_TIME, FIELD_PATH, FIELD_TITLE};
use crate::SearchHit;

/// 查询执行上下文
pub struct QueryContext<'a> {
    pub reader: &'a IndexReader,
    pub index: &'a Index,
    pub limit: usize,
}

/// 执行 Query AST 搜索
/// 
/// 搜索策略：
/// 1. 先用 Tantivy 执行全文关键词搜索，得到候选集
/// 2. 在候选集上应用各种过滤条件（glob、时间、大小等）
/// 3. 对 AND/OR/NOT 逻辑进行集合运算
pub fn execute_query(ctx: &QueryContext, query: &Query) -> Result<Vec<SearchHit>> {
    let schema = ctx.index.schema();
    
    // 收集所有关键词用于 Tantivy 搜索
    let keywords = collect_keywords(query);
    
    // 如果没有关键词，获取全部文档作为候选
    let candidates = if keywords.is_empty() {
        get_all_docs(ctx, &schema)?
    } else {
        // 构建关键词查询
        let query_str = keywords.join(" ");
        search_by_keywords(ctx, &query_str)?
    };
    
    // 在候选集上应用过滤器
    let filtered = filter_by_query(candidates, query)?;
    
    Ok(filtered)
}

/// 从 Query AST 中收集所有关键词
fn collect_keywords(query: &Query) -> Vec<String> {
    let mut keywords = Vec::new();
    collect_keywords_recursive(query, &mut keywords);
    keywords
}

fn collect_keywords_recursive(query: &Query, keywords: &mut Vec<String>) {
    match query {
        Query::Term(term) => {
            if let Term::KeyWord(kw) = term {
                keywords.push(kw.clone());
            }
        }
        Query::And(items) | Query::Or(items) => {
            for item in items {
                collect_keywords_recursive(item, keywords);
            }
        }
        Query::Not(_inner) => {
            // NOT 中的关键词不加入搜索，但需要在后处理中排除
        }
    }
}

/// 使用关键词进行 Tantivy 搜索
fn search_by_keywords(ctx: &QueryContext, query_str: &str) -> Result<Vec<SearchHit>> {
    let searcher = ctx.reader.searcher();
    let schema = ctx.index.schema();
    
    let title_field = schema.get_field(FIELD_TITLE)?;
    let body_field = schema.get_field(FIELD_BODY)?;
    let path_field = schema.get_field(FIELD_PATH)?;
    
    let query_parser = QueryParser::for_index(ctx.index, vec![title_field, body_field]);
    
    let tantivy_query = match query_parser.parse_query(query_str) {
        Ok(q) => q,
        Err(e) => {
            tracing::warn!("[Query执行器] 查询语法错误: '{}' - {}", query_str, e);
            return Ok(vec![]);
        }
    };
    
    let top_docs = searcher.search(&tantivy_query, &TopDocs::with_limit(ctx.limit * 10))?;
    
    let mut results = Vec::new();
    for (score, doc_address) in top_docs {
        let doc: TantivyDocument = searcher.doc(doc_address)?;
        
        let title = doc.get_first(title_field)
            .and_then(|v| v.as_str())
            .unwrap_or("无标题")
            .to_string();
        
        let path = doc.get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        // 获取文件元数据（如果有）
        let file_size = schema.get_field(FIELD_FILE_SIZE).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let modified_time = schema.get_field(FIELD_MODIFIED_TIME).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        results.push(SearchHit {
            title,
            path,
            score,
            tags: None,
            file_size,
            modified_time,
        });
    }
    
    Ok(results)
}

/// 获取所有文档（用于只有过滤条件没有关键词的情况）
fn get_all_docs(ctx: &QueryContext, schema: &tantivy::schema::Schema) -> Result<Vec<SearchHit>> {
    let searcher = ctx.reader.searcher();
    
    let title_field = schema.get_field(FIELD_TITLE)?;
    let path_field = schema.get_field(FIELD_PATH)?;
    
    let all_query = AllQuery;
    let top_docs = searcher.search(&all_query, &TopDocs::with_limit(ctx.limit * 10))?;
    
    let mut results = Vec::new();
    for (_score, doc_address) in top_docs {
        let doc: TantivyDocument = searcher.doc(doc_address)?;
        
        let title = doc.get_first(title_field)
            .and_then(|v| v.as_str())
            .unwrap_or("无标题")
            .to_string();
        
        let path = doc.get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let file_size = schema.get_field(FIELD_FILE_SIZE).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let modified_time = schema.get_field(FIELD_MODIFIED_TIME).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        results.push(SearchHit {
            title,
            path,
            score: 1.0,
            tags: None,
            file_size,
            modified_time,
        });
    }
    
    Ok(results)
}

/// 根据 Query AST 过滤候选结果
fn filter_by_query(candidates: Vec<SearchHit>, query: &Query) -> Result<Vec<SearchHit>> {
    let filtered: Vec<SearchHit> = candidates
        .into_iter()
        .filter(|hit| matches_query(hit, query))
        .collect();
    Ok(filtered)
}

/// 检查单个搜索结果是否匹配 Query
fn matches_query(hit: &SearchHit, query: &Query) -> bool {
    match query {
        Query::Term(term) => matches_term(hit, term),
        Query::And(items) => items.iter().all(|q| matches_query(hit, q)),
        Query::Or(items) => items.iter().any(|q| matches_query(hit, q)),
        Query::Not(inner) => !matches_query(hit, inner),
    }
}

/// 检查单个搜索结果是否匹配 Term
fn matches_term(hit: &SearchHit, term: &Term) -> bool {
    match term {
        Term::KeyWord(_) => {
            // 关键词已在 Tantivy 搜索中匹配，这里直接返回 true
            true
        }
        Term::Root(root_path) => {
            // 检查文件是否在指定根目录下
            let path = Path::new(&hit.path);
            let root = Path::new(root_path);
            path.starts_with(root)
        }
        Term::Regex(re) => {
            // 对路径或标题进行正则匹配
            re.is_match(&hit.path) || re.is_match(&hit.title)
        }
        Term::Glob(pattern) => {
            // Glob 模式匹配文件名
            match glob::Pattern::new(pattern) {
                Ok(p) => {
                    let file_name = Path::new(&hit.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    p.matches(file_name) || p.matches(&hit.path)
                }
                Err(_) => false,
            }
        }
        Term::AccessTime(range) => {
            // 访问时间过滤 - 从文件系统获取
            if let Ok(metadata) = std::fs::metadata(&hit.path) {
                if let Ok(accessed) = metadata.accessed() {
                    let secs = accessed
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    range.contains(secs)
                } else {
                    true // 无法获取时间，不过滤
                }
            } else {
                true
            }
        }
        Term::ModifiedTime(range) => {
            // 修改时间过滤
            let mtime_secs = if let Some(mtime) = hit.modified_time {
                mtime
            } else {
                // 如果没有元数据，尝试从文件系统获取
                if let Ok(metadata) = std::fs::metadata(&hit.path) {
                    if let Ok(modified) = metadata.modified() {
                        modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    } else {
                        return true; // 无法获取时间，不过滤
                    }
                } else {
                    return true;
                }
            };
            
            let result = range.contains(mtime_secs);
            tracing::debug!(
                "[ModifiedTime过滤] 文件: {}, mtime: {}, range: {:?}, 匹配: {}",
                hit.path, mtime_secs, range, result
            );
            result
        }
        Term::CreatedTime(range) => {
            // 创建时间过滤
            // TODO: 需要在 schema 中添加 created_time 字段
            if let Ok(metadata) = std::fs::metadata(&hit.path) {
                if let Ok(created) = metadata.created() {
                    let secs = created
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    range.contains(secs)
                } else {
                    true
                }
            } else {
                true
            }
        }
        Term::Size(range) => {
            // 文件大小过滤
            if let Some(size) = hit.file_size {
                range.contains(size)
            } else {
                // 从文件系统获取
                if let Ok(metadata) = std::fs::metadata(&hit.path) {
                    range.contains(metadata.len())
                } else {
                    true
                }
            }
        }
    }
}

/// 解析并执行查询字符串
/// 
/// 这是主要的入口函数，将原始查询字符串解析为 Query AST，然后执行搜索
pub fn parse_and_execute(
    reader: &IndexReader,
    index: &Index,
    query_str: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, QueryExecuteError> {
    // 1. 解析查询字符串
    let parsed = query::parse_query(query_str)
        .map_err(|e| QueryExecuteError::ParseError(format!("{:?}", e)))?;
    
    // 2. 验证并转换为 Query AST
    let query = query::validate_query(&parsed)
        .map_err(QueryExecuteError::ValidationError)?;
    
    tracing::debug!("[Query执行器] 解析后的 Query: {:?}", query);
    
    // 3. 执行查询
    let ctx = QueryContext { reader, index, limit };
    let results = execute_query(&ctx, &query)
        .map_err(|e| QueryExecuteError::ExecutionError(e.to_string()))?;
    
    // 4. 限制结果数量
    let results: Vec<_> = results.into_iter().take(limit).collect();
    
    Ok(results)
}

/// Query 执行错误
#[derive(Debug)]
pub enum QueryExecuteError {
    /// 解析错误
    ParseError(String),
    /// 验证错误
    ValidationError(ValidationError),
    /// 执行错误
    ExecutionError(String),
}

impl std::fmt::Display for QueryExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryExecuteError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            QueryExecuteError::ValidationError(e) => write!(f, "验证错误: {}", e),
            QueryExecuteError::ExecutionError(msg) => write!(f, "执行错误: {}", msg),
        }
    }
}

impl std::error::Error for QueryExecuteError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_collect_keywords() {
        let parsed = query::parse_query("foo AND bar").unwrap();
        let query = query::validate_query(&parsed).unwrap();
        let keywords = collect_keywords(&query);
        assert_eq!(keywords, vec!["foo", "bar"]);
    }
    
    #[test]
    fn test_collect_keywords_with_field() {
        let parsed = query::parse_query("keyword AND size:>1MB").unwrap();
        let query = query::validate_query(&parsed).unwrap();
        let keywords = collect_keywords(&query);
        assert_eq!(keywords, vec!["keyword"]);
    }
    
    #[test]
    fn test_collect_keywords_not_excluded() {
        let parsed = query::parse_query("foo AND NOT bar").unwrap();
        let query = query::validate_query(&parsed).unwrap();
        let keywords = collect_keywords(&query);
        // NOT 中的关键词不应该加入搜索
        assert_eq!(keywords, vec!["foo"]);
    }
}
