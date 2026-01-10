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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 搜索启动结果
#[derive(Debug, Serialize, Deserialize)]
pub enum StartSearchResult {
    /// 搜索已启动，后台异步执行
    Started { session_id: usize },
    /// 立即失败（参数错误等）
    Failed(String),
}

/// 搜索状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchStatus {
    /// 搜索进行中
    InProgress {
        /// 目前已找到的结果数
        found_so_far: usize,
    },
    /// 搜索已完成
    Completed {
        /// 总结果数
        total_count: usize,
    },
    /// 搜索失败
    Failed(String),
    /// 搜索已取消
    Cancelled,
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

/// Offset-based 结果获取响应
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResults {
    pub session_id: usize,
    /// 当前返回结果的起始偏移量
    pub offset: usize,
    /// 本次返回的结果
    pub hits: Vec<SearchHit>,
    /// 当前搜索状态
    pub status: SearchStatus,
    /// 是否还有更多结果（用于无限滚动）
    pub has_more: bool,
}

// ============ 兼容旧 API（可选保留）============

/// 旧版搜索结果（兼容）
#[derive(Debug, Serialize, Deserialize)]
pub enum SearchResult {
    /// Search started successfully
    Started { 
        session_id: usize,
        total_count: usize,
    },
    /// Immediate failure
    Failed(String)
}

/// 旧版分页结果（兼容）
#[derive(Debug, Serialize, Deserialize)]
pub struct PagedResults {
    pub session_id: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub hits: Vec<SearchHit>,
}
