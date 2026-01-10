use super::Command;
use crate::error::Result;
use crate::config::Config;
use crate::session::SessionManager;
use futures::{future, prelude::*};
use std::fs;
use std::sync::Arc;
use tracing::info;

use rpc::{
    World,
    search::{SearchRequest, SearchResult, StartSearchResult, FetchResults, PagedResults, SearchHit}
};
use tarpc::{
    context::Context,
    server::{self, Channel},
    tokio_serde::formats::Bincode
};

use search_core::{SearchConfig, SearchEngine, rpc_compat};

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

#[derive(Clone)]
struct Server {
    engine: Arc<SearchEngine>,
    sessions: Arc<SessionManager>,
}

impl Server {
    /// 将内部 SearchResultItem 转换为 RPC SearchHit
    fn convert_to_hits(results: Vec<rpc_compat::SearchResultItem>) -> Vec<SearchHit> {
        results.into_iter().map(|hit| {
            SearchHit {
                file_path: hit.path,
                score: hit.score,
                snippet: hit.preview,
                file_size: hit.file_size,
                modified_time: hit.modified_time,
            }
        }).collect()
    }
}

impl World for Server {
    async fn ping(self, _c: Context) -> String {
        "Pong".to_string()
    }

    // ============ 新 API: 异步流式搜索 ============

    async fn start_search_async(self, _c: Context, req: SearchRequest) -> StartSearchResult {
        info!("收到异步搜索请求: {:?}", req.keywords);
        
        // 创建异步会话（立即返回）
        let session_id = self.sessions.create_async_session();
        info!("创建异步搜索会话: {}", session_id);
        
        // 后台执行搜索
        let engine = self.engine.clone();
        let sessions = self.sessions.clone();
        let req_clone = req;
        
        let handle = tokio::spawn(async move {
            match rpc_compat::search_sync(&engine, &req_clone) {
                Ok(results) => {
                    let hits = Self::convert_to_hits(results);
                    info!("异步搜索完成，找到 {} 个结果", hits.len());
                    
                    // 追加结果并标记完成
                    sessions.append_results(session_id, hits);
                    sessions.mark_completed(session_id);
                }
                Err(e) => {
                    info!("异步搜索失败: {}", e);
                    sessions.mark_failed(session_id, e);
                }
            }
        });
        
        // 保存任务句柄（用于取消）
        self.sessions.set_task_handle(session_id, handle);
        
        StartSearchResult::Started { session_id }
    }

    async fn fetch_results(
        self, 
        _c: Context, 
        session_id: usize, 
        offset: usize, 
        limit: usize
    ) -> Option<FetchResults> {
        info!("获取搜索结果: session={}, offset={}, limit={}", session_id, offset, limit);
        self.sessions.fetch_results(session_id, offset, limit)
    }

    async fn cancel_search(self, _c: Context, session_id: usize) -> bool {
        info!("取消搜索会话: {}", session_id);
        self.sessions.cancel_session(session_id)
    }

    // ============ 旧 API: 同步搜索（兼容）============

    async fn start_search(self, _c: Context, req: SearchRequest) -> SearchResult {
        info!("收到同步搜索请求: {:?}", req.keywords);
        
        // 使用 rpc_compat 处理请求
        match rpc_compat::search_sync(&self.engine, &req) {
            Ok(results) => {
                let total_count = results.len();
                info!("搜索完成，找到 {} 个结果", total_count);
                
                // 转换为 SearchHit
                let hits = Self::convert_to_hits(results);
                
                // 创建会话存储结果
                let session_id = self.sessions.create_session(hits);
                info!("创建搜索会话: {}, 结果数: {}", session_id, total_count);
                
                SearchResult::Started { session_id, total_count }
            }
            Err(e) => {
                SearchResult::Failed(e)
            }
        }
    }

    async fn get_results_page(
        self, 
        _c: Context, 
        session_id: usize, 
        page: usize, 
        page_size: usize
    ) -> Option<PagedResults> {
        info!("获取分页结果: session={}, page={}, size={}", session_id, page, page_size);
        self.sessions.get_page(session_id, page, page_size)
    }
}

pub struct ServeCommand {
    config: Config
}

impl ServeCommand {
    pub fn new(cfg: Config) -> Self {
        Self {
            config: cfg
        }
    }
}

#[async_trait::async_trait]
impl Command for ServeCommand {
    async fn execute(&self) -> Result<()> {
        let unix_socket_path = self.config.runtime_dir.join(config::constants::UNIX_SOCKET_FILE_NAME);

        if let Some(parent) = unix_socket_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if unix_socket_path.exists() {
            fs::remove_file(&unix_socket_path)?;
        }

        info!("正在初始化搜索引擎...");
        
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
        let engine = Arc::new(
            SearchEngine::new(search_config)
                .map_err(|e| color_eyre::eyre::eyre!("创建搜索引擎失败: {}", e))?
        );
        
        // 创建会话管理器 (30分钟超时)
        let sessions = Arc::new(SessionManager::new(1800));
        
        info!("搜索引擎初始化完成");
        info!("监听 {:?}", unix_socket_path);

        let mut listener = tarpc::serde_transport::unix::listen(&unix_socket_path, Bincode::default).await?;
        listener.config_mut().max_frame_length(usize::MAX);

        let server = Server { engine, sessions };

        listener
            .filter_map(|r| future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .map(|channel| {
                let server = server.clone();
                channel.execute(server.serve()).for_each(spawn)
            })
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;
        
        Ok(())
    }
}
