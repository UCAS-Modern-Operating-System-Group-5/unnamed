use super::Command;
use crate::error::Result;
use crate::settings::Settings;

pub struct ServeCommand {
    settings: Settings
}

impl ServeCommand {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings
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
