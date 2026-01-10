// search-core/src/lib.rs
//! 搜索引擎核心库
//!
//! 提供基于 Tantivy 的全文搜索引擎，支持：
//! - AI 关键词提取 (BERT)
//! - 文件元数据索引
//! - 实时文件监控
//! - 增量索引

pub mod ai;
pub mod cache;
pub mod config;
pub mod extract;
pub mod indexer;
pub mod models;
pub mod registry;
pub mod search;

// RPC 适配层（可选功能）
#[cfg(feature = "rpc-compat")]
pub mod rpc_compat;

// 重导出核心类型
pub use ai::{BertModel, KeywordExtractor};
pub use cache::{EmbeddingCache, FileMetaEntry, FileStatus};
pub use config::{SearchConfig, IndexConfig, AiConfig, WalkerConfig};
pub use extract::{extract_text, TextExtractor};
pub use indexer::{
    init_persistent_index, 
    scan_existing_files, 
    delete_from_index,
    start_file_watcher,
};
pub use models::FileDoc;
pub use registry::{FileRegistry, FileState, EventType, PendingEvent};
pub use search::search_index;

/// 搜索引擎统一入口
pub struct SearchEngine {
    pub index: tantivy::Index,
    pub schema: tantivy::schema::Schema,
    pub reader: tantivy::IndexReader,
    pub bert: BertModel,
    pub cache: EmbeddingCache,
    pub registry: FileRegistry,
    pub config: SearchConfig,
}

impl SearchEngine {
    /// 创建搜索引擎实例
    pub fn new(config: SearchConfig) -> anyhow::Result<Self> {
        use std::path::Path;
        
        // 初始化索引
        let storage_path = Path::new(&config.index.storage_path);
        let (index, schema, reader) = init_persistent_index(storage_path)?;
        
        // 加载 AI 模型
        println!(" [AI] 正在加载 BERT 模型 (首次运行需下载)...");
        let bert = BertModel::new()?;
        println!(" [AI] 模型加载完毕！");
        
        // 初始化缓存
        let cache_path = Path::new(&config.cache_path);
        let cache = EmbeddingCache::new(cache_path)?;
        let (count, size) = cache.stats();
        println!(" [Cache] 缓存统计: {} 条记录, {} 字节", count, size);
        
        // 创建注册表
        let registry = FileRegistry::new();
        
        Ok(Self {
            index,
            schema,
            reader,
            bert,
            cache,
            registry,
            config,
        })
    }
    
    /// 执行搜索
    pub fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>> {
        search::search_with_results(&self.reader, &self.index, query)
    }
    
    /// 使用 AI 优化查询
    pub fn refine_query(&self, query: &str) -> String {
        self.bert.refine_query(query)
    }
    
    /// 索引单个文件
    pub fn index_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        indexer::process_and_index(path, &self.index, &self.schema, &self.bert, &self.cache)
    }
    
    /// 删除文件索引
    pub fn delete_file(&self, path: &std::path::Path) -> anyhow::Result<bool> {
        delete_from_index(path, &self.index, &self.schema, Some(&self.cache))
    }
    
    /// 扫描并索引目录
    pub fn scan_directory(&self, watch_path: &std::path::Path) -> anyhow::Result<()> {
        scan_existing_files(
            watch_path,
            &self.index,
            &self.schema,
            &self.bert,
            &self.cache,
            &self.registry,
        )
    }
}

/// 搜索结果
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub title: String,
    pub path: String,
    pub score: f32,
    pub tags: Option<String>,
}
