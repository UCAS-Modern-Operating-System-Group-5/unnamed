pub mod serve;
pub mod index;
pub mod clear_cache;

use crate::error::Result;

pub use serve::ServeCommand;
pub use index::IndexCommand;
pub use clear_cache::ClearCacheCommand;

#[async_trait::async_trait]
pub trait Command {
    async fn execute(&self) -> Result<()>;
}
