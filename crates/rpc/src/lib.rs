pub mod search;

use search::{SearchRequest, StartSearchResult, FetchResults, PagedResults};

#[tarpc::service]
pub trait World {
    /// Heartbeat
    async fn ping() -> String;

    // ============ 新 API: Offset-based 流式搜索 ============
    
    /// 启动搜索（立即返回，后台异步执行）
    async fn start_search_async(req: SearchRequest) -> StartSearchResult;
    
    /// 获取结果（offset-based，支持流式/无限滚动）
    /// - offset: 从第几个结果开始
    /// - limit: 最多返回多少个
    async fn fetch_results(session_id: usize, offset: usize, limit: usize) -> Option<FetchResults>;
    
    /// 取消搜索并释放资源
    async fn cancel_search(session_id: usize) -> bool;
    
    // ============ 旧 API: 兼容 ============
    
    /// Start search process (同步，等待全部完成)
    async fn start_search(req: SearchRequest) -> search::SearchResult;
    
    /// Get paginated results for a search session (page-based)
    async fn get_results_page(session_id: usize, page: usize, page_size: usize) -> Option<PagedResults>;
}

#[derive(Debug)]
pub enum Request {
    Ping,
    StartSearch(SearchRequest)
}

#[derive(Debug)]
pub enum Response {
    Ping(String),
    StartSearch(search::SearchResult)
}
