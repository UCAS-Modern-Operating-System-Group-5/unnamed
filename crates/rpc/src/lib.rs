pub mod search;

use search::{SearchRequest, SearchResult, PagedResults};

#[tarpc::service]
pub trait World {
    /// Heartbeat
    async fn ping() -> String;

    /// Start search process
    async fn start_search(req: SearchRequest) -> SearchResult;
    
    /// Get paginated results for a search session
    async fn get_results_page(session_id: usize, page: usize, page_size: usize) -> Option<PagedResults>;
    
    /// Cancel a search session and free resources
    async fn cancel_search(session_id: usize) -> bool;
}

#[derive(Debug)]
pub enum Request {
    Ping,
    StartSearch(SearchRequest)
}

#[derive(Debug)]
pub enum Response {
    Ping(String),
    StartSearch(SearchResult)
}
