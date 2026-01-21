// search-core/src/lib.rs
//! 搜索引擎核心库
//!
//! 提供基于 Tantivy 的全文搜索引擎，支持：
//! - AI 关键词提取 (BERT)
//! - 文件元数据索引
//! - 实时文件监控
//! - 增量索引

use std::sync::Arc;

pub mod ai;
pub mod cache;
pub mod config;
pub mod extract;
pub mod indexer;
pub mod models;
pub mod registry;
pub mod schema;
pub mod search;
pub mod query_executor;

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
pub use schema::{build_schema, IndexDocument, SchemaFields, FIELD_TITLE, FIELD_BODY, FIELD_PATH, FIELD_TAGS, FIELD_FILE_SIZE, FIELD_MODIFIED_TIME};
pub use search::search_index;
pub use query_executor::{execute_query, parse_and_execute, QueryContext, QueryExecuteError};

/// 搜索引擎统一入口
pub struct SearchEngine {
    pub index: tantivy::Index,
    pub schema: tantivy::schema::Schema,
    pub reader: tantivy::IndexReader,
    pub bert: Arc<BertModel>,
    pub cache: Arc<EmbeddingCache>,
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
        let bert = Arc::new(BertModel::new()?);
        println!(" [AI] 模型加载完毕！");
        
        // 初始化缓存
        let cache_path = Path::new(&config.cache_path);
        let cache = Arc::new(EmbeddingCache::new(cache_path)?);
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
    
    /// 执行搜索（传统全文搜索）
    pub fn search(&self, query: &str) -> anyhow::Result<Vec<SearchHit>> {
        search::search_with_results(&self.reader, &self.index, query)
    }
    
    /// 混合搜索：结合传统全文搜索和语义向量搜索
    /// 
    /// # 参数
    /// - `query`: 搜索查询字符串
    /// - `use_semantic`: 是否使用语义搜索
    /// - `text_weight`: 传统搜索权重（0.0-1.0）
    /// - `semantic_weight`: 语义搜索权重（0.0-1.0）
    /// - `limit`: 返回结果数量上限
    pub fn hybrid_search(
        &self,
        query: &str,
        use_semantic: bool,
        text_weight: f32,
        semantic_weight: f32,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>> {
        if !use_semantic {
            // 只使用传统搜索
            let mut results = self.search(query)?;
            results.truncate(limit);
            return Ok(results);
        }
        
        // 获取查询的向量表示
        let query_embedding = self.bert.get_embedding(query).ok();
        
        search::hybrid_search(
            &self.reader,
            &self.index,
            query,
            query_embedding.as_deref(),
            text_weight,
            semantic_weight,
            limit,
        )
    }
    
    /// 使用 AI 优化查询
    pub fn refine_query(&self, query: &str) -> String {
        let refined = self.bert.refine_query(query);
        tracing::debug!("[AI 查询优化] 原始查询: '{}'", query);
        tracing::debug!("[AI 查询优化] 优化后: '{}'", refined);
        refined
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
    /// 文件大小（字节），可选
    pub file_size: Option<u64>,
    /// 修改时间（Unix 时间戳秒），可选
    pub modified_time: Option<u64>,
    /// 创建时间（Unix 时间戳秒），可选
    pub created_time: Option<u64>,
    /// 访问时间（Unix 时间戳秒），可选
    pub accessed_time: Option<u64>,
}
