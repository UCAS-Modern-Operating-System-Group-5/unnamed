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
    let searcher = reader.searcher();
    
    let schema = index.schema();
    let title_field = schema.get_field(FIELD_TITLE).unwrap();
    let body_field = schema.get_field(FIELD_BODY).unwrap();
    let path_field = schema.get_field(FIELD_PATH).unwrap();
    let tags_field = schema.get_field(FIELD_TAGS).ok();

    let query_parser = QueryParser::for_index(index, vec![title_field, body_field]);
    
    let query = match query_parser.parse_query(query_str) {
        Ok(q) => q,
        Err(e) => {
            tracing::warn!("查询语法错误: {}", e);
            return Ok(vec![]);
        }
    };

    let top_docs = searcher.search(&query, &TopDocs::with_limit(20))?;
    
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
        
        results.push(SearchHit {
            title,
            path,
            score,
            tags,
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
