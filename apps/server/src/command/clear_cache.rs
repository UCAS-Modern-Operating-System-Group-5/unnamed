// apps/server/src/command/clear_cache.rs
//! 清除缓存命令

use super::Command;
use crate::config::Config;
use crate::error::Result;
use std::fs;
use std::path::Path;

pub struct ClearCacheCommand {
    config: Config,
}

impl ClearCacheCommand {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn remove_dir_contents(path: &Path, name: &str) -> Result<(usize, u64)> {
        if !path.exists() {
            println!("  📁 {} 不存在，跳过", name);
            return Ok((0, 0));
        }

        let mut file_count = 0;
        let mut total_size = 0u64;

        // 计算目录大小
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        total_size += metadata.len();
                        file_count += 1;
                    } else if metadata.is_dir() {
                        // 递归计算子目录大小
                        if let Ok((sub_count, sub_size)) = Self::count_dir_size(&entry.path()) {
                            file_count += sub_count;
                            total_size += sub_size;
                        }
                    }
                }
            }
        }

        // 删除目录
        fs::remove_dir_all(path)?;
        
        Ok((file_count, total_size))
    }

    fn count_dir_size(path: &Path) -> Result<(usize, u64)> {
        let mut file_count = 0;
        let mut total_size = 0u64;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        total_size += metadata.len();
                        file_count += 1;
                    } else if metadata.is_dir() {
                        if let Ok((sub_count, sub_size)) = Self::count_dir_size(&entry.path()) {
                            file_count += sub_count;
                            total_size += sub_size;
                        }
                    }
                }
            }
        }

        Ok((file_count, total_size))
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

#[async_trait::async_trait]
impl Command for ClearCacheCommand {
    async fn execute(&self) -> Result<()> {
        println!("\n🗑️  清除缓存");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        let cache_dir = &self.config.cache_dir;
        let index_path = cache_dir.join("index");
        let embedding_cache_path = cache_dir.join("embedding_cache");

        println!("📂 缓存目录: {:?}\n", cache_dir);

        let mut total_files = 0;
        let mut total_bytes = 0u64;

        // 清除 Tantivy 索引
        print!("  🔍 Tantivy 索引 (index/)... ");
        match Self::remove_dir_contents(&index_path, "index") {
            Ok((count, size)) => {
                if count > 0 || size > 0 {
                    println!("✅ 已删除 {} 个文件, {}", count, Self::format_size(size));
                    total_files += count;
                    total_bytes += size;
                } else {
                    println!("⏭️  目录为空或不存在");
                }
            }
            Err(e) => println!("❌ 失败: {}", e),
        }

        // 清除 Embedding 缓存
        print!("  🧠 Embedding 缓存 (embedding_cache/)... ");
        match Self::remove_dir_contents(&embedding_cache_path, "embedding_cache") {
            Ok((count, size)) => {
                if count > 0 || size > 0 {
                    println!("✅ 已删除 {} 个文件, {}", count, Self::format_size(size));
                    total_files += count;
                    total_bytes += size;
                } else {
                    println!("⏭️  目录为空或不存在");
                }
            }
            Err(e) => println!("❌ 失败: {}", e),
        }

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("✨ 清理完成！共删除 {} 个文件，释放 {}", total_files, Self::format_size(total_bytes));
        println!("\n💡 提示: 运行 'cargo run -p server -- index <路径>' 重新建立索引");

        Ok(())
    }
}
