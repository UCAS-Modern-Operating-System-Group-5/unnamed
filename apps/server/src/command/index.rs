use super::Command;
use crate::error::Result;
use crate::config::Config;
use std::path::PathBuf;
use tracing::info;

use search_core::{SearchConfig, SearchEngine};

pub struct IndexCommand {
    config: Config,
    root_path: Option<PathBuf>,
}

impl IndexCommand {
    pub fn new(cfg: Config, root_path: Option<PathBuf>) -> Self {
        Self {
            config: cfg,
            root_path
        }
    }
}

#[async_trait::async_trait]
impl Command for IndexCommand {
    async fn execute(&self) -> Result<()> {
        // 构建搜索引擎配置
        let search_config = SearchConfig {
            watch_paths: self.config.watch_paths.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            index: search_core::IndexConfig {
                storage_path: self.config.cache_dir.join("index").to_string_lossy().to_string(),
                writer_memory: 50_000_000,
            },
            ai: search_core::AiConfig {
                model_path: self.config.cache_dir.join("model").to_string_lossy().to_string(),
                keyword_count: 3,
            },
            cache_path: self.config.cache_dir.join("embedding_cache").to_string_lossy().to_string(),
            ..Default::default()
        };
        
        // 创建搜索引擎
        let engine = SearchEngine::new(search_config)
            .map_err(|e| color_eyre::eyre::eyre!("创建搜索引擎失败: {}", e))?;
        
        // 确定要索引的路径
        let paths_to_index = if let Some(ref path) = self.root_path {
            vec![path.clone()]
        } else {
            self.config.watch_paths.clone()
        };
        
        if paths_to_index.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "没有指定要索引的目录。\n\n请使用以下方式之一：\n\
                1. 命令行参数: cargo run -p server -- index <目录路径>\n\
                2. 配置文件: 在 ~/.config/mcst/unnamed/server.toml 中设置 watch-paths"
            ).into());
        }
        
        // 扫描并索引每个目录
        for path in &paths_to_index {
            info!("开始索引目录: {:?}", path);
            engine.scan_directory(path)
                .map_err(|e| color_eyre::eyre::eyre!("索引目录失败: {}", e))?;
        }
        
        info!("所有目录索引完成");
        Ok(())
    }
}

