pub mod serve;
pub mod index;

use crate::error::Result;

pub use serve::ServeCommand;
pub use index::IndexCommand;

#[async_trait::async_trait]
pub trait Command {
    async fn execute(&self) -> Result<()>;
}
