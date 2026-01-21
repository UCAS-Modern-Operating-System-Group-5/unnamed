use super::Command;
use crate::error::Result;
use crate::config::Config;
use crate::session::SessionManager;
use futures::{future, prelude::*};
use std::fs;
use std::sync::Arc;
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

use rpc::{
    World,
    search::{
        SearchRequest, FetchSearchResultsRequest, FetchResults, 
        SearchHit, SearchStatus, SearchErrorKind, SResult, SearchMode
    }
};
use tarpc::{
    context::Context,
    server::{self, Channel},
    tokio_serde::formats::Bincode
};

use search_core::{SearchConfig, SearchEngine, rpc_compat, start_file_watcher};

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
            let modified_secs = hit.modified_time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            let created_secs = hit.created_time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            let accessed_secs = hit.accessed_time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            
            SearchHit {
                file_path: hit.path,
                score: Some(hit.score),
                preview: hit.preview,
                file_size: hit.file_size,
                access_time: accessed_secs,
                modified_time: modified_secs,
                create_time: created_secs,
            }
        }).collect()
    }
}

impl World for Server {
    async fn ping(self, _c: Context) -> String {
        "Pong".to_string()
    }

    async fn start_search(self, _c: Context, req: SearchRequest) -> SResult<Uuid> {
        info!("收到搜索请求: query='{}', mode={:?}", req.query, req.search_mode);
        
        if req.query.is_empty() {
            return Err(SearchErrorKind::InvalidQuery(
                query::ValidationError::new(
                    query::empty_span(),
                    query::ValidationErrorKind::EmptyValue,
                )
            ));
        }
        
        // 创建异步会话（立即返回）
        let session_id = self.sessions.create_async_session();
        info!("创建搜索会话: {}", session_id);
        
        // 后台执行搜索
        let engine = self.engine.clone();
        let sessions = self.sessions.clone();
        let query_str = req.query.clone();
        let search_mode = req.search_mode;
        
        let handle = tokio::spawn(async move {
            let limit = 1000;  // 默认最大结果数
            
            let result = match search_mode {
                SearchMode::Rule => {
                    rpc_compat::search_with_query_dsl(&engine, &query_str, limit)
                }
                SearchMode::Natural => {
                    rpc_compat::search_with_semantic(&engine, &query_str, limit)
                }
            };
            
            match result {
                Ok(results) => {
                    let hits = Self::convert_to_hits(results);
                    info!("搜索完成，找到 {} 个结果", hits.len());
                    
                    // 追加结果并标记完成
                    sessions.append_results(session_id, hits);
                    sessions.mark_completed(session_id);
                }
                Err(e) => {
                    info!("搜索失败: {}", e);
                    // 将错误转换为 SearchErrorKind
                    let error_kind = match e {
                        rpc_compat::QuerySearchError::ParseError(msg) |
                        rpc_compat::QuerySearchError::ValidationError(msg) => {
                            SearchErrorKind::InvalidQuery(
                                query::ValidationError::new(
                                    query::empty_span(),
                                    query::ValidationErrorKind::InvalidRange { reason: msg },
                                )
                            )
                        }
                        rpc_compat::QuerySearchError::ExecutionError(_) => {
                            SearchErrorKind::OperateOnAlreadyFailedSearch
                        }
                    };
                    sessions.mark_failed(session_id, error_kind);
                }
            }
        });
        
        // 保存任务句柄（用于取消）
        self.sessions.set_task_handle(session_id, handle);
        
        Ok(session_id)
    }

    async fn search_status(self, _c: Context, session_id: Uuid) -> (Uuid, SResult<SearchStatus>) {
        info!("查询搜索状态: session={}", session_id);
        
        let res = self.sessions.get_status(session_id).ok_or(SearchErrorKind::SessionNotExists);
        (session_id, res)
    }

    async fn fetch_search_results(
        self, 
        _c: Context, 
        req: FetchSearchResultsRequest,
    ) -> (Uuid, SResult<FetchResults>) {
        info!("获取搜索结果: session={}, offset={}, limit={}", 
              req.session_id, req.offset, req.limit);
        
        let res = self.sessions.fetch_results(req.session_id, req.offset, req.limit)
            .ok_or(SearchErrorKind::SessionNotExists);

        (req.session_id, res)
    }

    async fn cancel_search(self, _c: Context, session_id: Uuid) -> (Uuid, SResult<()>) {
        info!("取消搜索会话: {}", session_id);
        
        let res = if self.sessions.cancel_session(session_id) {
            Ok(())
        } else {
            Err(SearchErrorKind::SessionNotExists)
        };

        (session_id, res)
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
        
        // 启动文件监控（如果配置了 watch_paths）
        let _watcher_handles: Vec<_> = self.config.watch_paths.iter()
            .filter(|p| p.exists())
            .map(|watch_path| {
                info!("启动文件监控: {:?}", watch_path);
                
                let scan_complete_tx = start_file_watcher(
                    watch_path.clone(),
                    engine.index.clone(),
                    engine.schema.clone(),
                    engine.bert.clone(),
                    engine.cache.clone(),
                    engine.registry.clone(),
                );
                
                // 执行初始扫描
                let _ = engine.scan_directory(watch_path);
                
                // 通知监控线程扫描完成
                let _ = scan_complete_tx.send(());
                
                scan_complete_tx
            })
            .collect();
        
        if self.config.watch_paths.is_empty() {
            info!("⚠️  未配置 watch-paths，文件监控未启动");
            info!("💡 编辑 ~/.config/unnamed/server.toml 添加要监控的目录");
        }
        
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
