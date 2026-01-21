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

use crate::schema::{FIELD_BODY, FIELD_FILE_SIZE, FIELD_MODIFIED_TIME, FIELD_CREATED_TIME, FIELD_ACCESSED_TIME, FIELD_PATH, FIELD_TITLE};
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
    tracing::info!("[Query执行器] 收集到关键词: {:?}", keywords);
    
    // 如果没有关键词，获取全部文档作为候选
    let candidates = if keywords.is_empty() {
        tracing::info!("[Query执行器] 无关键词，获取全部文档作为候选");
        get_all_docs(ctx, &schema)?
    } else {
        // 构建关键词查询
        let query_str = keywords.join(" ");
        tracing::info!("[Query执行器] 使用关键词搜索: '{}'", query_str);
        search_by_keywords(ctx, &query_str)?
    };
    
    tracing::info!("[Query执行器] 候选文档数: {}", candidates.len());
    
    // 在候选集上应用过滤器
    let filtered = filter_by_query(candidates, query)?;
    
    tracing::info!("[Query执行器] 过滤后结果数: {}", filtered.len());
    
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
            match term {
                Term::KeyWord(kw) => {
                    keywords.push(kw.clone());
                }
                Term::Regex(re) => {
                    // 将正则表达式的模式作为关键词用于 Tantivy 搜索
                    // Tantivy 会对其进行分词并在 body 中搜索
                    let pattern = re.as_str();
                    if !pattern.is_empty() {
                        keywords.push(pattern.to_string());
                    }
                }
                _ => {}
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
        
        let created_time = schema.get_field(FIELD_CREATED_TIME).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let accessed_time = schema.get_field(FIELD_ACCESSED_TIME).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        results.push(SearchHit {
            title,
            path,
            score,
            tags: None,
            file_size,
            modified_time,
            created_time,
            accessed_time,
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
    let fetch_limit = ctx.limit * 10;
    tracing::info!("[Query执行器] get_all_docs: 获取所有文档，limit={}", fetch_limit);
    let top_docs = searcher.search(&all_query, &TopDocs::with_limit(fetch_limit))?;
    tracing::info!("[Query执行器] get_all_docs: 获取到 {} 个候选文档", top_docs.len());
    
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
        
        let created_time = schema.get_field(FIELD_CREATED_TIME).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        let accessed_time = schema.get_field(FIELD_ACCESSED_TIME).ok()
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_u64());
        
        results.push(SearchHit {
            title,
            path,
            score: 1.0,
            tags: None,
            file_size,
            modified_time,
            created_time,
            accessed_time,
        });
    }
    
    Ok(results)
}

/// 根据 Query AST 过滤候选结果
fn filter_by_query(candidates: Vec<SearchHit>, query: &Query) -> Result<Vec<SearchHit>> {
    let candidate_count = candidates.len();
    let filtered: Vec<SearchHit> = candidates
        .into_iter()
        .filter(|hit| matches_query(hit, query))
        .collect();
    tracing::info!(
        "[Query执行器] filter_by_query: 候选 {} 个, 过滤后 {} 个",
        candidate_count, filtered.len()
    );
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
        Term::Regex(_re) => {
            // Regex 模式已作为关键词传给 Tantivy 进行全文搜索
            // Tantivy 会在 title 和 body 中搜索匹配的内容
            // 这里直接返回 true，因为候选结果已经是 Tantivy 匹配的
            true
        }
        Term::Glob(pattern) => {
            // Glob 模式匹配文件名
            match glob::Pattern::new(pattern) {
                Ok(p) => {
                    let file_name = Path::new(&hit.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    let matches_name = p.matches(file_name);
                    let matches_path = p.matches(&hit.path);
                    let result = matches_name || matches_path;
                    tracing::debug!(
                        "[Glob过滤] pattern='{}', file_name='{}', path='{}', matches_name={}, matches_path={}, result={}",
                        pattern, file_name, hit.path, matches_name, matches_path, result
                    );
                    result
                }
                Err(e) => {
                    tracing::warn!("[Glob过滤] 无效的 glob 模式 '{}': {}", pattern, e);
                    false
                }
            }
        }
        Term::AccessTime(range) => {
            // 访问时间过滤 - 优先使用索引中的数据
            let atime_secs = if let Some(atime) = hit.accessed_time {
                atime
            } else {
                // 如果没有元数据，尝试从文件系统获取
                if let Ok(metadata) = std::fs::metadata(&hit.path) {
                    if let Ok(accessed) = metadata.accessed() {
                        accessed
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
            
            let result = range.contains(atime_secs);
            tracing::debug!(
                "[AccessTime过滤] 文件: {}, atime: {}, range: {:?}, 匹配: {}",
                hit.path, atime_secs, range, result
            );
            result
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
            // 创建时间过滤 - 优先使用索引中的数据
            let ctime_secs = if let Some(ctime) = hit.created_time {
                ctime
            } else {
                // 如果没有元数据，尝试从文件系统获取
                if let Ok(metadata) = std::fs::metadata(&hit.path) {
                    if let Ok(created) = metadata.created() {
                        created
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
            
            let result = range.contains(ctime_secs);
            tracing::debug!(
                "[CreatedTime过滤] 文件: {}, ctime: {}, range: {:?}, 匹配: {}",
                hit.path, ctime_secs, range, result
            );
            result
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
