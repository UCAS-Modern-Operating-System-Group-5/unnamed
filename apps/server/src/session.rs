//! 搜索会话管理模块
//! 
//! 支持两种模式:
//! 1. 同步模式: 创建会话时直接传入所有结果
//! 2. 异步模式: 后台任务逐步追加结果，客户端可立即开始获取

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use rpc::search::{SearchHit, PagedResults, FetchResults, SearchStatus};
use tokio::task::JoinHandle;

/// 搜索会话
pub struct SearchSession {
    pub session_id: usize,
    /// 结果缓冲区（生产者可追加）
    pub results: Vec<SearchHit>,
    /// 搜索状态
    pub status: SearchStatus,
    /// 后台任务句柄（可取消）
    pub task_handle: Option<JoinHandle<()>>,
    pub created_at: Instant,
    pub last_accessed: Instant,
}

impl SearchSession {
    /// 创建新会话（异步模式，初始为空）
    pub fn new_async(session_id: usize) -> Self {
        Self {
            session_id,
            results: Vec::new(),
            status: SearchStatus::InProgress { found_so_far: 0 },
            task_handle: None,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
        }
    }
    
    /// 创建新会话（同步模式，直接传入结果）
    pub fn new_sync(session_id: usize, results: Vec<SearchHit>) -> Self {
        let total_count = results.len();
        Self {
            session_id,
            results,
            status: SearchStatus::Completed { total_count },
            task_handle: None,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
        }
    }
}

/// 会话管理器
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<usize, SearchSession>>>,
    next_session_id: Arc<RwLock<usize>>,
    session_timeout: Duration,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new(session_timeout_seconds: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(RwLock::new(1)),
            session_timeout: Duration::from_secs(session_timeout_seconds),
        }
    }

    /// 生成下一个会话 ID
    fn next_id(&self) -> usize {
        let mut id = self.next_session_id.write().unwrap();
        let current_id = *id;
        *id += 1;
        current_id
    }

    /// 创建新的搜索会话（同步模式 - 直接传入所有结果）
    pub fn create_session(&self, results: Vec<SearchHit>) -> usize {
        let session_id = self.next_id();
        let session = SearchSession::new_sync(session_id, results);
        self.sessions.write().unwrap().insert(session_id, session);
        self.cleanup_expired_sessions();
        session_id
    }

    /// 创建异步搜索会话（立即返回，后台搜索）
    pub fn create_async_session(&self) -> usize {
        let session_id = self.next_id();
        let session = SearchSession::new_async(session_id);
        self.sessions.write().unwrap().insert(session_id, session);
        self.cleanup_expired_sessions();
        session_id
    }

    /// 追加搜索结果（用于异步模式）
    pub fn append_results(&self, session_id: usize, hits: Vec<SearchHit>) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.results.extend(hits);
            // 更新状态
            if let SearchStatus::InProgress { .. } = session.status {
                session.status = SearchStatus::InProgress { 
                    found_so_far: session.results.len() 
                };
            }
        }
    }

    /// 标记搜索完成
    pub fn mark_completed(&self, session_id: usize) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.status = SearchStatus::Completed { 
                total_count: session.results.len() 
            };
        }
    }

    /// 标记搜索失败
    pub fn mark_failed(&self, session_id: usize, error: String) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.status = SearchStatus::Failed(error);
        }
    }

    /// 设置后台任务句柄（用于取消）
    pub fn set_task_handle(&self, session_id: usize, handle: JoinHandle<()>) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.task_handle = Some(handle);
        }
    }

    /// 获取结果（offset-based，支持流式）
    pub fn fetch_results(&self, session_id: usize, offset: usize, limit: usize) -> Option<FetchResults> {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_accessed = Instant::now();
            
            let current_count = session.results.len();
            
            // 从缓冲区取 [offset, offset+limit) 范围
            let end = std::cmp::min(offset + limit, current_count);
            let hits = if offset < current_count {
                session.results[offset..end].to_vec()
            } else {
                vec![]
            };
            
            // 判断是否还有更多
            let has_more = match &session.status {
                SearchStatus::InProgress { .. } => true,  // 还在搜，肯定有更多
                SearchStatus::Completed { total_count } => offset + hits.len() < *total_count,
                SearchStatus::Failed(_) => false,
                SearchStatus::Cancelled => false,
            };
            
            Some(FetchResults {
                session_id,
                offset,
                hits,
                status: session.status.clone(),
                has_more,
            })
        } else {
            None
        }
    }

    /// 获取分页结果（旧 API 兼容）
    pub fn get_page(&self, session_id: usize, page: usize, page_size: usize) -> Option<PagedResults> {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_accessed = Instant::now();
            
            let total_count = session.results.len();
            let total_pages = (total_count + page_size - 1) / page_size;
            
            let start = page * page_size;
            let end = std::cmp::min(start + page_size, total_count);
            
            let hits = if start < total_count {
                session.results[start..end].to_vec()
            } else {
                vec![]
            };
            
            Some(PagedResults {
                session_id,
                page,
                page_size,
                total_count,
                total_pages,
                hits,
            })
        } else {
            None
        }
    }

    /// 取消搜索会话
    pub fn cancel_session(&self, session_id: usize) -> bool {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            // 取消后台任务
            if let Some(handle) = session.task_handle.take() {
                handle.abort();
            }
            session.status = SearchStatus::Cancelled;
        }
        
        sessions.remove(&session_id).is_some()
    }

    /// 获取会话总结果数
    pub fn get_total_count(&self, session_id: usize) -> Option<usize> {
        self.sessions.read().unwrap()
            .get(&session_id)
            .map(|s| s.results.len())
    }

    /// 获取会话状态
    pub fn get_status(&self, session_id: usize) -> Option<SearchStatus> {
        self.sessions.read().unwrap()
            .get(&session_id)
            .map(|s| s.status.clone())
    }

    /// 清理过期会话
    fn cleanup_expired_sessions(&self) {
        let now = Instant::now();
        let timeout = self.session_timeout;
        
        let mut sessions = self.sessions.write().unwrap();
        let to_remove: Vec<usize> = sessions.iter()
            .filter(|(_, session)| now.duration_since(session.last_accessed) >= timeout)
            .map(|(id, _)| *id)
            .collect();
        
        for id in to_remove {
            if let Some(mut session) = sessions.remove(&id) {
                // 取消后台任务
                if let Some(handle) = session.task_handle.take() {
                    handle.abort();
                }
            }
        }
    }

    /// 获取活跃会话数
    pub fn active_sessions_count(&self) -> usize {
        self.cleanup_expired_sessions();
        self.sessions.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn create_mock_hit(path: &str, score: f32) -> SearchHit {
        SearchHit {
            file_path: PathBuf::from(path),
            score,
            snippet: "test snippet".to_string(),
            file_size: 1024,
            modified_time: SystemTime::now(),
        }
    }

    #[test]
    fn test_sync_session() {
        let manager = SessionManager::new(300);
        let hits = vec![
            create_mock_hit("/path/1.txt", 0.9),
            create_mock_hit("/path/2.txt", 0.8),
            create_mock_hit("/path/3.txt", 0.7),
        ];
        
        let session_id = manager.create_session(hits);
        
        // 获取全部结果
        let result = manager.fetch_results(session_id, 0, 10).unwrap();
        assert_eq!(result.hits.len(), 3);
        assert!(!result.has_more);
        assert!(matches!(result.status, SearchStatus::Completed { total_count: 3 }));
    }

    #[test]
    fn test_async_session() {
        let manager = SessionManager::new(300);
        
        // 创建异步会话
        let session_id = manager.create_async_session();
        
        // 初始状态
        let result = manager.fetch_results(session_id, 0, 10).unwrap();
        assert_eq!(result.hits.len(), 0);
        assert!(result.has_more);  // 还在进行中
        
        // 追加结果
        manager.append_results(session_id, vec![
            create_mock_hit("/path/1.txt", 0.9),
            create_mock_hit("/path/2.txt", 0.8),
        ]);
        
        // 获取已有结果
        let result = manager.fetch_results(session_id, 0, 10).unwrap();
        assert_eq!(result.hits.len(), 2);
        assert!(result.has_more);  // 还在进行中
        
        // 标记完成
        manager.mark_completed(session_id);
        
        let result = manager.fetch_results(session_id, 0, 10).unwrap();
        assert_eq!(result.hits.len(), 2);
        assert!(!result.has_more);  // 完成了
    }

    #[test]
    fn test_offset_pagination() {
        let manager = SessionManager::new(300);
        let hits: Vec<SearchHit> = (0..25)
            .map(|i| create_mock_hit(&format!("/path/{}.txt", i), 1.0 - i as f32 * 0.01))
            .collect();
        
        let session_id = manager.create_session(hits);
        
        // 第一批 (0-9)
        let result = manager.fetch_results(session_id, 0, 10).unwrap();
        assert_eq!(result.hits.len(), 10);
        assert_eq!(result.offset, 0);
        assert!(result.has_more);
        
        // 第二批 (10-19)
        let result = manager.fetch_results(session_id, 10, 10).unwrap();
        assert_eq!(result.hits.len(), 10);
        assert_eq!(result.offset, 10);
        assert!(result.has_more);
        
        // 第三批 (20-24)
        let result = manager.fetch_results(session_id, 20, 10).unwrap();
        assert_eq!(result.hits.len(), 5);
        assert_eq!(result.offset, 20);
        assert!(!result.has_more);  // 没有更多了
    }

    #[test]
    fn test_session_cancel() {
        let manager = SessionManager::new(300);
        let hits = vec![create_mock_hit("/path/1.txt", 0.9)];
        
        let session_id = manager.create_session(hits);
        assert!(manager.cancel_session(session_id));
        assert!(manager.fetch_results(session_id, 0, 10).is_none());
    }
}
