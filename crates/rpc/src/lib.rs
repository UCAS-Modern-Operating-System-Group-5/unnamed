pub mod search;

use search::{
    FetchResults, FetchSearchResultsRequest, SearchRequest, SearchStatus,
    SResult
};
use uuid::Uuid;

#[tarpc::service]
pub trait World {
    /// Heartbeat
    async fn ping() -> String;

    /// Start async search
    async fn start_search(req: SearchRequest) -> SResult<Uuid>;

    async fn search_status(session_id: Uuid) -> SResult<SearchStatus>;

    async fn fetch_search_results(
        req: FetchSearchResultsRequest,
    ) -> SResult<FetchResults>;

    async fn cancel_search(session_id: Uuid) -> SResult<()>;
}

#[derive(Debug)]
pub enum Request {
    Ping,
    StartSearch(SearchRequest),
    SearchStatus(Uuid),
    FetchSearchResults(FetchSearchResultsRequest),
    CancelSearch(Uuid)
}

#[derive(Debug)]
pub enum Response {
    Ping(String),
    StartSearch(SResult<Uuid>),
    SearchStatus(SResult<SearchStatus>),
    FetchSearchResults(SResult<FetchResults>),
    CancelSearch(SResult<()>)
}
