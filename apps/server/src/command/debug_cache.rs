// apps/server/src/command/debug_cache.rs
//! 调试缓存命令 - 查看 BERT 提取的关键词

use super::Command;
use crate::config::Config;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct DebugCacheCommand {
    config: Config,
    filter: Option<String>,
    limit: usize,
}

/// 缓存条目结构（与 search-core/src/cache.rs 保持一致）
#[derive(Serialize, Deserialize, Debug)]
struct CacheEntry {
    content_hash: u64,
    keywords: Vec<String>,
}

/// 文件元数据缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMetaEntry {
    file_size: u64,
    mtime: u64,
    indexed: bool,
}

impl DebugCacheCommand {
    pub fn new(config: Config, filter: Option<String>, limit: usize) -> Self {
        Self { config, filter, limit }
    }

    fn format_size(size: u64) -> String {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.2} KB", size as f64 / 1024.0)
        } else {
            format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
        }
    }

    fn format_time(timestamp: u64) -> String {
        use chrono::{TimeZone, Utc, Local};
        match Local.timestamp_opt(timestamp as i64, 0) {
            chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            _ => format!("时间戳: {}", timestamp),
        }
    }
}

#[async_trait::async_trait]
impl Command for DebugCacheCommand {
    async fn execute(&self) -> Result<()> {
        let cache_dir = &self.config.cache_dir;
        let embedding_cache_path = cache_dir.join("embedding_cache");

        println!("🔍 Embedding 缓存调试工具");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📂 缓存目录: {:?}", embedding_cache_path);
        
        if let Some(ref filter) = self.filter {
            println!("🔎 过滤条件: {}", filter);
        }
        println!();

        if !embedding_cache_path.exists() {
            println!("❌ 缓存目录不存在，请先运行索引命令");
            return Ok(());
        }

        // 尝试打开 sled 数据库
        let db = match sled::open(&embedding_cache_path) {
            Ok(db) => db,
            Err(e) => {
                println!("⚠️  无法打开缓存数据库: {}", e);
                println!();
                println!("💡 提示: 数据库可能被其他进程（如正在运行的 server）锁定。");
                println!("   请先停止 server 服务后再运行此命令。");
                println!();
                println!("   停止 server: pkill -f 'server serve'");
                println!("   或: killall server");
                return Ok(());
            }
        };
        
        let mut keyword_count = 0;
        let mut meta_count = 0;
        let mut displayed = 0;

        println!("📋 关键词缓存列表:");
        println!("────────────────────────────────────────────────────────────");

        for item in db.iter() {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);

            // 跳过元数据条目
            if key_str.starts_with("meta:") {
                meta_count += 1;
                continue;
            }

            // 应用过滤条件
            if let Some(ref filter) = self.filter {
                if !key_str.contains(filter) {
                    keyword_count += 1;
                    continue;
                }
            }

            // 解析缓存条目
            match bincode::deserialize::<CacheEntry>(&value) {
                Ok(entry) => {
                    if displayed < self.limit {
                        println!();
                        println!("📄 文件: {}", key_str);
                        println!("   🏷️  关键词: {:?}", entry.keywords);
                        println!("   #️⃣  内容哈希: {:016x}", entry.content_hash);
                        
                        // 检查文件是否存在
                        let path = Path::new(key_str.as_ref());
                        if path.exists() {
                            println!("   ✅ 文件存在");
                        } else {
                            println!("   ⚠️  文件已删除（孤儿缓存）");
                        }
                        
                        displayed += 1;
                    }
                    keyword_count += 1;
                }
                Err(e) => {
                    println!("   ❌ 解析失败: {}", e);
                }
            }
        }

        println!();
        println!("────────────────────────────────────────────────────────────");
        println!("📊 统计信息:");
        println!("   • 关键词缓存条目: {}", keyword_count);
        println!("   • 文件元数据条目: {}", meta_count);
        println!("   • 显示条目数: {} / {}", displayed, self.limit);
        
        // 数据库大小
        if let Ok(size) = db.size_on_disk() {
            println!("   • 数据库大小: {}", Self::format_size(size));
        }

        println!();
        println!("💡 提示:");
        println!("   • 使用 --filter <关键词> 过滤文件路径");
        println!("   • 使用 --limit <数量> 限制显示条目数");
        println!("   • 使用 --show-meta 显示文件元数据");

        Ok(())
    }
}

/// 带元数据显示的调试命令
pub struct DebugCacheMetaCommand {
    config: Config,
    filter: Option<String>,
    limit: usize,
}

impl DebugCacheMetaCommand {
    pub fn new(config: Config, filter: Option<String>, limit: usize) -> Self {
        Self { config, filter, limit }
    }
}

#[async_trait::async_trait]
impl Command for DebugCacheMetaCommand {
    async fn execute(&self) -> Result<()> {
        let cache_dir = &self.config.cache_dir;
        let embedding_cache_path = cache_dir.join("embedding_cache");

        println!("🔍 文件元数据缓存调试工具");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📂 缓存目录: {:?}", embedding_cache_path);
        println!();

        if !embedding_cache_path.exists() {
            println!("❌ 缓存目录不存在");
            return Ok(());
        }

        let db = match sled::open(&embedding_cache_path) {
            Ok(db) => db,
            Err(e) => {
                println!("⚠️  无法打开缓存数据库: {}", e);
                println!();
                println!("💡 提示: 数据库可能被其他进程（如正在运行的 server）锁定。");
                println!("   请先停止 server 服务后再运行此命令。");
                return Ok(());
            }
        };
        
        let mut displayed = 0;
        let meta_prefix = b"meta:";

        println!("📋 文件元数据列表:");
        println!("────────────────────────────────────────────────────────────");

        for item in db.scan_prefix(meta_prefix) {
            let (key, value) = item?;
            let key_str = String::from_utf8_lossy(&key);
            let file_path = key_str.strip_prefix("meta:").unwrap_or(&key_str);

            // 应用过滤条件
            if let Some(ref filter) = self.filter {
                if !file_path.contains(filter) {
                    continue;
                }
            }

            if displayed >= self.limit {
                break;
            }

            match bincode::deserialize::<FileMetaEntry>(&value) {
                Ok(meta) => {
                    println!();
                    println!("📄 文件: {}", file_path);
                    println!("   📏 大小: {}", DebugCacheCommand::format_size(meta.file_size));
                    println!("   🕐 修改时间: {}", DebugCacheCommand::format_time(meta.mtime));
                    println!("   📝 已索引: {}", if meta.indexed { "是" } else { "否" });
                    
                    let path = Path::new(file_path);
                    if path.exists() {
                        println!("   ✅ 文件存在");
                    } else {
                        println!("   ⚠️  文件已删除");
                    }
                    
                    displayed += 1;
                }
                Err(e) => {
                    println!("   ❌ 解析失败: {}", e);
                }
            }
        }

        println!();
        println!("────────────────────────────────────────────────────────────");
        println!("📊 显示条目数: {}", displayed);

        Ok(())
    }
}
