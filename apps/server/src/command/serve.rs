use super::Command;
use crate::error::Result;
use crate::config::Config;

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
        println!("Serving...");
        Ok(())
    }
}
