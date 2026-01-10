// In the future, this may be moved to `ipc` crate
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub root_directories: Vec<PathBuf>,

    // === Search Queries ===
    pub regular_expressions: Vec<String>,
    pub keywords: Vec<String>,
    /// Natural language queries for the local AI model
    /// e.g., "Find the report about project alpha from last week"
    pub semantic_queries: Vec<String>,
    
    /// Minimum similarity score for semantic search (0.0 to 1.0)
    /// Useful to filter out irrelevant AI results.
    pub semantic_threshold: Option<f32>,
    
    // === Filters ===
    /// Glob patterns to include (e.g., "*.txt", "*.rs")
    pub include_globs: Vec<String>,
    /// Glob patterns to exclude (e.g., "target/", ".git/")
    pub exclude_globs: Vec<String>,

    pub time_accessed_range: Option<(SystemTime, SystemTime)>,
    pub time_created_range: Option<(SystemTime, SystemTime)>,
    pub time_modified_range: Option<(SystemTime, SystemTime)>,
    /// File size range in bytes
    pub size_range_bytes: Option<(u64, u64)>,
    
    // === Presentation & Control ===
    pub sort: SortMode,
    pub max_results: Option<usize>
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SortMode {
    Alphabetical,
    ReverseAlphabetical,
    AccessedTime,
    CreatedTime,
    ModifiedTime,
    Extension,
    /// Sort by AI relevance score
    Relevance,
}


#[derive(Debug, Serialize, Deserialize)]
pub enum SearchResult {
    /// Search started successfully
    /// session_id allows the client to listen for specific result streams
    Started { 
        session_id: usize,
        total_count: usize,  // 总结果数
    },
    
    /// Immediate failure (e.g., Invalid Regex, Path not found).
    Failed(String)
}

/// 单个搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub file_path: PathBuf,
    pub score: f32,
    pub snippet: String,
    pub file_size: u64,
    pub modified_time: SystemTime,
}

/// 分页结果
#[derive(Debug, Serialize, Deserialize)]
pub struct PagedResults {
    pub session_id: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub hits: Vec<SearchHit>,
}
