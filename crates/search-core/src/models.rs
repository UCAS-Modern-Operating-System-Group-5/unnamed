// search-core/src/models.rs
//! 数据模型定义

use serde::{Deserialize, Serialize};

/// 文件文档结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDoc {
    pub title: String,
    pub content: String,
    pub path: String,
}

impl FileDoc {
    pub fn new(title: impl Into<String>, content: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            path: path.into(),
        }
    }
}
