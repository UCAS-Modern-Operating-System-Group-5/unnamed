// In the future, this may be moved to `ipc` crate
use std::path::PathBuf;

pub struct SearchRequest {
    pub root_directories: Vec<PathBuf>,
    pub query: String,
    regular_expressions: Vec<String>,
    keywords: Vec<String>
}

pub enum SearchResult {
    Success,
    /// Search failed message with reason
    Failed(String)
}
