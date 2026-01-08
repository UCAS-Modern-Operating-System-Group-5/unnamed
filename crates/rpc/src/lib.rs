pub mod search;

use search::{SearchRequest, SearchResult};

#[tarpc::service]
pub trait World {
    /// Heartbeat
    async fn ping() -> String;

    /// Start search process
    async fn start_search(req: SearchRequest) -> SearchResult;
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
