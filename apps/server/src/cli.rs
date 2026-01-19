use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Increase verbosity. Can be used multiple times (e.g., -v, -vv, -vvv).
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Serve
    Serve,
    /// Index the whole file system
    Index {
        /// 要索引的目录路径（可选，不提供则使用配置文件中的 watch_paths）
        root_path: Option<PathBuf>
    },
    /// Clear all caches (index + embedding cache)
    ClearCache,
    /// Debug: 查看缓存的关键词和元数据
    DebugCache {
        /// 过滤文件路径（支持部分匹配）
        #[arg(short, long)]
        filter: Option<String>,
        /// 限制显示的条目数量
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// 显示文件元数据而非关键词
        #[arg(long)]
        show_meta: bool,
    },
}
