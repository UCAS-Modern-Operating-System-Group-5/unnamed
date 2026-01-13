// search-core/src/schema/document.rs
//! 文档结构定义
//! 
//! 定义索引文档的结构化表示，用于创建 Tantivy 文档

use std::path::Path;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// 索引文档 - 待写入 Tantivy 的文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDocument {
    // ============== 已启用字段 ==============
    
    /// 文件标题（通常是文件名，不含扩展名）
    pub title: String,
    
    /// 文件内容
    pub content: String,
    
    /// 完整文件路径
    pub path: String,
    
    /// AI 生成的标签
    pub tags: Vec<String>,
    
    /// 文件大小（字节）
    pub file_size: u64,
    
    /// 修改时间（Unix 时间戳秒）
    pub modified_time: u64,
    
    // ============== 待启用字段 ==============
    // 取消注释并在 builder.rs 中添加对应字段即可启用
    
    // /// 父目录路径
    // pub parent_path: String,
    
    // /// 文件名（含扩展名）
    // pub filename: String,
    
    // /// 文件类型/扩展名
    // pub file_type: String,
    
    // /// 创建时间（Unix 时间戳秒）
    // pub created_time: u64,
    
    // /// 访问时间（Unix 时间戳秒）
    // pub accessed_time: u64,
    
    // /// 索引时间（Unix 时间戳秒）
    // pub indexed_time: u64,
}

impl IndexDocument {
    /// 从文件路径创建索引文档（不含 AI 标签）
    pub fn from_path(path: &Path, title: String, content: String) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        let path_str = canonical_path.to_string_lossy().to_string();
        
        let file_size = metadata.len();
        
        let modified_time = metadata.modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // === 待启用字段的提取逻辑 ===
        // let parent_path = canonical_path.parent()
        //     .map(|p| p.to_string_lossy().to_string())
        //     .unwrap_or_default();
        // 
        // let filename = canonical_path.file_name()
        //     .map(|n| n.to_string_lossy().to_string())
        //     .unwrap_or_default();
        // 
        // let file_type = canonical_path.extension()
        //     .map(|e| e.to_string_lossy().to_lowercase())
        //     .unwrap_or_default();
        // 
        // let created_time = metadata.created()
        //     .unwrap_or(SystemTime::UNIX_EPOCH)
        //     .duration_since(SystemTime::UNIX_EPOCH)
        //     .unwrap_or_default()
        //     .as_secs();
        // 
        // let accessed_time = metadata.accessed()
        //     .unwrap_or(SystemTime::UNIX_EPOCH)
        //     .duration_since(SystemTime::UNIX_EPOCH)
        //     .unwrap_or_default()
        //     .as_secs();
        // 
        // let indexed_time = SystemTime::now()
        //     .duration_since(SystemTime::UNIX_EPOCH)
        //     .unwrap_or_default()
        //     .as_secs();
        
        Ok(Self {
            title,
            content,
            path: path_str,
            tags: Vec::new(),
            file_size,
            modified_time,
            // parent_path,
            // filename,
            // file_type,
            // created_time,
            // accessed_time,
            // indexed_time,
        })
    }
    
    /// 设置 AI 标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    /// 获取标签字符串（空格分隔，用于索引）
    pub fn tags_string(&self) -> String {
        self.tags.join(" ")
    }
}
