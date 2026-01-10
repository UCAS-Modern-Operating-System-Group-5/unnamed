// search-core/src/config.rs
//! 配置模块

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 搜索引擎配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    /// 要监控和索引的目录列表
    pub watch_paths: Vec<String>,
    pub index: IndexConfig,
    pub ai: AiConfig,
    pub walker: WalkerConfig,
    pub cache_path: String,
    pub display: DisplayConfig,
}

/// 索引配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IndexConfig {
    pub storage_path: String,
    pub writer_memory: usize,
}

/// AI 配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    pub model_path: String,
    pub keyword_count: usize,
}

/// Walker 配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WalkerConfig {
    pub use_ripgrep_walker: bool,
    pub respect_gitignore: bool,
    pub respect_ignore: bool,
    pub skip_hidden: bool,
    pub follow_symlinks: bool,
    pub max_depth: usize,
    pub custom_ignore_patterns: Vec<String>,
    pub supported_extensions: Vec<String>,
}

/// 显示配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    pub preview_max_length: usize,
    pub sentence_search_start: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec!["./docs".to_string()],
            index: IndexConfig::default(),
            ai: AiConfig::default(),
            walker: WalkerConfig::default(),
            cache_path: "./cache".to_string(),
            display: DisplayConfig::default(),
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            storage_path: "./storage".to_string(),
            writer_memory: 50_000_000,
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model_path: "./model".to_string(),
            keyword_count: 3,
        }
    }
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            use_ripgrep_walker: true,
            respect_gitignore: true,
            respect_ignore: true,
            skip_hidden: true,
            follow_symlinks: false,
            max_depth: 0,
            custom_ignore_patterns: vec![
                "*.log".to_string(),
                "*.tmp".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
            ],
            supported_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "pdf".to_string(),
            ],
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            preview_max_length: 200,
            sentence_search_start: 50,
        }
    }
}

impl SearchConfig {
    /// 从 TOML 文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: SearchConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 尝试加载配置，失败则使用默认值
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        Self::load_from_file(path).unwrap_or_default()
    }
}

/// 全局配置（用于兼容旧代码）
pub static CONFIG: Lazy<SearchConfig> = Lazy::new(|| {
    SearchConfig::load_or_default("./config.toml")
});
