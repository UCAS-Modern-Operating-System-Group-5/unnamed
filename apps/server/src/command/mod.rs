pub mod serve;

use crate::error::Result;

pub use serve::ServeCommand;

#[async_trait::async_trait]
pub trait Command {
    async fn execute(&self) -> Result<()>;
}
