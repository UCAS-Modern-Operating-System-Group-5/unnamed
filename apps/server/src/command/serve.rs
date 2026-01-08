use super::Command;
use crate::error::Result;
use crate::config::Config;
use futures::{future, prelude::*};
use std::fs;
use tracing::info;

use rpc::{
    Request, Response, ServeWorld, World,
    search::{SearchRequest , SearchResult}
};
use tarpc::{
    context::Context,
    server::{self, Channel, incoming::Incoming},
    tokio_serde::formats::Bincode
};



async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

#[derive(Clone)]
struct Server;

impl World for Server {
    async fn ping(self, _c: Context) -> String {
        "Pong".to_string()
    }

    async fn start_search(self, _c: Context, _req: SearchRequest) -> SearchResult {
        SearchResult::Failed("Not implemented yet".into())
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

        info!("Listening on {:?}", unix_socket_path);

        let mut listener = tarpc::serde_transport::unix::listen(&unix_socket_path, Bincode::default).await?;
        listener.config_mut().max_frame_length(usize::MAX);

        let server = Server {};

        listener
            // Ignore accept errors.
            .filter_map(|r| future::ready(r.ok()))
            .map(server::BaseChannel::with_defaults)
            .map(|channel| {
                let server = server.clone();
                channel.execute(server.serve()).for_each(spawn)
            })
            // Max 10 concurrent connections.
            .buffer_unordered(10)
            .for_each(|_| async {})
            .await;
        
        Ok(())
    }
}
