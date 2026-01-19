//! 搜索会话管理模块
//! 
//! 支持基于 UUID 的会话管理，配合新的 RPC schema

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;
use rpc::search::{SearchHit, FetchResults, SearchStatus, SearchErrorKind};
use tokio::task::JoinHandle;

/// 搜索会话
pub struct SearchSession {
    pub session_id: Uuid,
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
    pub fn new_async(session_id: Uuid) -> Self {
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
    pub fn new_sync(session_id: Uuid, results: Vec<SearchHit>) -> Self {
        let total_count = results.len() as u64;
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
    sessions: Arc<RwLock<HashMap<Uuid, SearchSession>>>,
    session_timeout: Duration,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new(session_timeout_seconds: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout: Duration::from_secs(session_timeout_seconds),
        }
    }

    /// 创建异步搜索会话（立即返回，后台搜索）
    pub fn create_async_session(&self) -> Uuid {
        let session_id = Uuid::new_v4();
        let session = SearchSession::new_async(session_id);
        self.sessions.write().unwrap().insert(session_id, session);
        self.cleanup_expired_sessions();
        session_id
    }

    /// 检查会话是否存在
    pub fn session_exists(&self, session_id: Uuid) -> bool {
        self.sessions.read().unwrap().contains_key(&session_id)
    }

    /// 追加搜索结果（用于异步模式）
    pub fn append_results(&self, session_id: Uuid, hits: Vec<SearchHit>) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.results.extend(hits);
            // 更新状态
            if let SearchStatus::InProgress { .. } = session.status {
                session.status = SearchStatus::InProgress { 
                    found_so_far: session.results.len() as u64
                };
            }
        }
    }

    /// 标记搜索完成
    pub fn mark_completed(&self, session_id: Uuid) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.status = SearchStatus::Completed { 
                total_count: session.results.len() as u64
            };
        }
    }

    /// 标记搜索失败
    pub fn mark_failed(&self, session_id: Uuid, error: SearchErrorKind) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.status = SearchStatus::Failed(error);
        }
    }

    /// 设置后台任务句柄（用于取消）
    pub fn set_task_handle(&self, session_id: Uuid, handle: JoinHandle<()>) {
        if let Some(session) = self.sessions.write().unwrap().get_mut(&session_id) {
            session.task_handle = Some(handle);
        }
    }

    /// 获取会话状态
    pub fn get_status(&self, session_id: Uuid) -> Option<SearchStatus> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_accessed = Instant::now();
            Some(session.status.clone())
        } else {
            None
        }
    }

    /// 获取结果（offset-based，支持流式）
    pub fn fetch_results(&self, session_id: Uuid, offset: usize, limit: usize) -> Option<FetchResults> {
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
                SearchStatus::Completed { total_count } => offset + hits.len() < (*total_count) as usize,
                SearchStatus::Failed(_) => false,
                SearchStatus::Cancelled => false,
            };
            
            Some(FetchResults {
                session_id,
                offset: offset as u64,
                hits,
                has_more,
            })
        } else {
            None
        }
    }

    /// 取消搜索会话
    pub fn cancel_session(&self, session_id: Uuid) -> bool {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            // 取消后台任务
            if let Some(handle) = session.task_handle.take() {
                handle.abort();
            }
            session.status = SearchStatus::Cancelled;
            true
        } else {
            false
        }
    }

    /// 删除会话
    pub fn remove_session(&self, session_id: Uuid) -> bool {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(mut session) = sessions.remove(&session_id) {
            if let Some(handle) = session.task_handle.take() {
                handle.abort();
            }
            true
        } else {
            false
        }
    }

    /// 清理过期会话
    fn cleanup_expired_sessions(&self) {
        let now = Instant::now();
        let timeout = self.session_timeout;
        
        let mut sessions = self.sessions.write().unwrap();
        let to_remove: Vec<Uuid> = sessions.iter()
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

    fn create_mock_hit(path: &str, score: f32) -> SearchHit {
        SearchHit {
            file_path: PathBuf::from(path),
            score: Some(score),
            preview: "test snippet".to_string(),
            file_size: 1024,
            access_time: 0,
            modified_time: 0,
            create_time: 0,
        }
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
        
        let session_id = manager.create_async_session();
        
        // 追加 25 个结果
        let hits: Vec<SearchHit> = (0..25)
            .map(|i| create_mock_hit(&format!("/path/{}.txt", i), 1.0 - i as f32 * 0.01))
            .collect();
        manager.append_results(session_id, hits);
        manager.mark_completed(session_id);
        
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
        
        let session_id = manager.create_async_session();
        manager.append_results(session_id, vec![create_mock_hit("/path/1.txt", 0.9)]);
        
        assert!(manager.cancel_session(session_id));
        
        let status = manager.get_status(session_id).unwrap();
        assert!(matches!(status, SearchStatus::Cancelled));
    }
}
