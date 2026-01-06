use super::Command;
use crate::error::Result;
use crate::config::Config;
use std::path::PathBuf;

pub struct IndexCommand {
    config: Config,
    root_path: PathBuf,
}

impl IndexCommand {
    pub fn new(cfg: Config, root_path: PathBuf) -> Self {
        Self {
            config: cfg,
            root_path
        }
    }
}

#[async_trait::async_trait]
impl Command for IndexCommand {
    async fn execute(&self) -> Result<()> {
        println!("Indexing...");
        Ok(())
    }
}

