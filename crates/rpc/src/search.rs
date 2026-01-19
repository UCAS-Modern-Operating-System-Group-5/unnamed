use serde::{Deserialize, Serialize};
use strum::EnumIter;
use std::path::PathBuf;
use std::time::SystemTime;
use uuid::Uuid;
use query::ValidationError;

pub type SResult<T> = Result<T, SearchErrorKind>;

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub search_mode: SearchMode,
}


#[derive(Debug, Default, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum SearchMode {
    /// Natural language
    #[default]
    Natural,
    /// Rule based search (i.e. exact match, regexp, specifying root directory)
    Rule,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct FetchSearchResultsRequest {
    pub session_id: Uuid,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchErrorKind {
    SessionNotExists,
    SessionAlreadyCancelled,
    InvalidQuery(ValidationError),
    OperateOnAlreadyFailedSearch,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchStatus {
    InProgress {
        found_so_far: u64,
    },
    Completed {
        total_count: u64,
    },
    /// Could be like `invalid query`
    Failed(SearchErrorKind),
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResults {
    pub session_id: Uuid,
    pub offset: u64,
    pub hits: Vec<SearchHit>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub file_path: PathBuf,
    /// Score is only available for natural language search 
    pub score: Option<f32>,
    pub preview: String,
    pub file_size: u64,
    /// Access time since unix epoch
    pub access_time: u64,
    pub modified_time: u64,
    pub create_time: u64
}

