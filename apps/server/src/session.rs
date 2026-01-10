//! 搜索会话管理模块
//! 
//! 负责管理搜索结果的持久化和分页访问

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use rpc::search::{SearchHit, PagedResults};

/// 搜索会话
#[derive(Debug, Clone)]
pub struct SearchSession {
    pub session_id: usize,
    pub results: Vec<SearchHit>,
    pub created_at: Instant,
    pub last_accessed: Instant,
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

    /// 创建新的搜索会话
    pub fn create_session(&self, results: Vec<SearchHit>) -> usize {
        let session_id = {
            let mut id = self.next_session_id.write().unwrap();
            let current_id = *id;
            *id += 1;
            current_id
        };

        let session = SearchSession {
            session_id,
            results,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
        };

        self.sessions.write().unwrap().insert(session_id, session);
        
        // 清理过期会话
        self.cleanup_expired_sessions();
        
        session_id
    }

    /// 获取分页结果
    pub fn get_page(&self, session_id: usize, page: usize, page_size: usize) -> Option<PagedResults> {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            // 更新最后访问时间
            session.last_accessed = Instant::now();
            
            let total_count = session.results.len();
            let total_pages = (total_count + page_size - 1) / page_size;
            
            // 计算分页范围
            let start = page * page_size;
            let end = std::cmp::min(start + page_size, total_count);
            
            if start >= total_count {
                return Some(PagedResults {
                    session_id,
                    page,
                    page_size,
                    total_count,
                    total_pages,
                    hits: vec![],
                });
            }
            
            let hits = session.results[start..end].to_vec();
            
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
        self.sessions.write().unwrap().remove(&session_id).is_some()
    }

    /// 获取会话总结果数
    pub fn get_total_count(&self, session_id: usize) -> Option<usize> {
        self.sessions.read().unwrap()
            .get(&session_id)
            .map(|s| s.results.len())
    }

    /// 清理过期会话
    fn cleanup_expired_sessions(&self) {
        let now = Instant::now();
        let timeout = self.session_timeout;
        
        self.sessions.write().unwrap().retain(|_, session| {
            now.duration_since(session.last_accessed) < timeout
        });
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
    fn test_session_creation() {
        let manager = SessionManager::new(300);
        let hits = vec![
            create_mock_hit("/path/1.txt", 0.9),
            create_mock_hit("/path/2.txt", 0.8),
        ];
        
        let session_id = manager.create_session(hits);
        assert_eq!(session_id, 1);
        assert_eq!(manager.get_total_count(session_id), Some(2));
    }

    #[test]
    fn test_pagination() {
        let manager = SessionManager::new(300);
        let hits: Vec<SearchHit> = (0..25)
            .map(|i| create_mock_hit(&format!("/path/{}.txt", i), 1.0 - (i as f32) * 0.01))
            .collect();
        
        let session_id = manager.create_session(hits);
        
        // 第一页 (10 个结果)
        let page1 = manager.get_page(session_id, 0, 10).unwrap();
        assert_eq!(page1.hits.len(), 10);
        assert_eq!(page1.total_count, 25);
        assert_eq!(page1.total_pages, 3);
        
        // 第三页 (5 个结果)
        let page3 = manager.get_page(session_id, 2, 10).unwrap();
        assert_eq!(page3.hits.len(), 5);
    }

    #[test]
    fn test_session_cancel() {
        let manager = SessionManager::new(300);
        let hits = vec![create_mock_hit("/path/1.txt", 0.9)];
        
        let session_id = manager.create_session(hits);
        assert!(manager.cancel_session(session_id));
        assert!(manager.get_page(session_id, 0, 10).is_none());
    }
}
