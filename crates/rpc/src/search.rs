use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    query: String,
    search_mode: SearchMode,
    sort_mode: SortMode,
}

#[derive(
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumCount,
    Clone,
)]
pub enum SortMode {
    #[default]
    Alphabetical,
    ReverseAlphabetical,
    AccessedTime,
    CreatedTime,
    ModifiedTime,
    Extension,
    /// Sort by AI relevance score
    Relevance,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum SearchMode {
    /// Natural language
    #[default]
    Natural,
    /// Rule based search (i.e. exact match, regexp, specifying root directory)
    Rule,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SearchResult {
    /// Search started successfully
    /// session_id allows the client to listen for specific result streams
    Started { session_id: usize },

    /// Immediate failure (e.g., Invalid Regex, Path not found).
    Failed(String),
}
