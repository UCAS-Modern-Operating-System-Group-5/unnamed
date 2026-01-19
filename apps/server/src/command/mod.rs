pub mod serve;
pub mod index;
pub mod clear_cache;
pub mod debug_cache;

use crate::error::Result;

pub use serve::ServeCommand;
pub use index::IndexCommand;
pub use clear_cache::ClearCacheCommand;
pub use debug_cache::{DebugCacheCommand, DebugCacheMetaCommand};

#[async_trait::async_trait]
pub trait Command {
    async fn execute(&self) -> Result<()>;
}
