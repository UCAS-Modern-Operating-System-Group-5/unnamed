// search-core/src/registry.rs
//! 文件注册表 - 协调扫描和监听之间的同步

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// 文件状态
#[derive(Debug, Clone)]
pub struct FileState {
    pub modified_time: SystemTime,
    pub processed_time: SystemTime,
    pub processing: bool,
}

/// 文件注册表 - 线程安全的文件状态管理
#[derive(Clone)]
pub struct FileRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

struct RegistryInner {
    files: HashMap<PathBuf, FileState>,
    scan_completed: bool,
    pending_events: Vec<PendingEvent>,
}

#[derive(Debug, Clone)]
pub struct PendingEvent {
    pub path: PathBuf,
    pub event_type: EventType,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Create,
    Modify,
    Delete,
}

impl FileRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                files: HashMap::new(),
                scan_completed: false,
                pending_events: Vec::new(),
            })),
        }
    }

    /// 尝试开始处理文件（原子操作）
    pub fn try_start_processing(&self, path: &PathBuf, file_mod_time: SystemTime) -> bool {
        let mut inner = self.inner.write().unwrap();
        
        if let Some(state) = inner.files.get_mut(path) {
            if state.processing {
                return false;
            }
            if state.modified_time >= file_mod_time {
                return false;
            }
            state.processing = true;
            state.modified_time = file_mod_time;
            true
        } else {
            inner.files.insert(path.clone(), FileState {
                modified_time: file_mod_time,
                processed_time: SystemTime::now(),
                processing: true,
            });
            true
        }
    }

    /// 完成处理文件
    pub fn finish_processing(&self, path: &PathBuf) {
        let mut inner = self.inner.write().unwrap();
        if let Some(state) = inner.files.get_mut(path) {
            state.processing = false;
            state.processed_time = SystemTime::now();
        }
    }

    /// 标记文件已删除
    pub fn mark_deleted(&self, path: &PathBuf) {
        let mut inner = self.inner.write().unwrap();
        inner.files.remove(path);
    }

    /// 添加待处理事件（扫描期间使用）
    pub fn add_pending_event(&self, path: PathBuf, event_type: EventType) {
        let mut inner = self.inner.write().unwrap();
        if !inner.scan_completed {
            inner.pending_events.push(PendingEvent {
                path,
                event_type,
                timestamp: SystemTime::now(),
            });
        }
    }

    /// 标记扫描完成，返回待处理的事件
    pub fn complete_scan(&self) -> Vec<PendingEvent> {
        let mut inner = self.inner.write().unwrap();
        inner.scan_completed = true;
        std::mem::take(&mut inner.pending_events)
    }

    /// 检查扫描是否完成
    pub fn is_scan_completed(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.scan_completed
    }

    /// 检查文件是否已被处理
    pub fn is_file_processed(&self, path: &PathBuf, file_mod_time: SystemTime) -> bool {
        let inner = self.inner.read().unwrap();
        if let Some(state) = inner.files.get(path) {
            state.modified_time >= file_mod_time
        } else {
            false
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> (usize, usize) {
        let inner = self.inner.read().unwrap();
        let processing_count = inner.files.values().filter(|s| s.processing).count();
        (inner.files.len(), processing_count)
    }
}

impl Default for FileRegistry {
    fn default() -> Self {
        Self::new()
    }
}
