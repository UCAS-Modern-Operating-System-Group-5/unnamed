pub mod constants;

pub use etcetera::AppStrategy;
use etcetera::{AppStrategyArgs, choose_app_strategy};

use std::path::PathBuf;
use std::env;


pub fn create_strategy() -> std::result::Result<impl AppStrategy, etcetera::HomeDirError> {
    choose_app_strategy(AppStrategyArgs {
        top_level_domain: constants::TOP_LEVEL_DOMAIN.to_string(),
        author: constants::AUTHOR.to_string(),
        app_name: constants::APP_NAME.to_string(),
    })
}

pub fn resolve_dir<S, F>(env_key: &str, strategy: &S, strategy_fn: F) -> PathBuf
where
    S: AppStrategy,
    F: FnOnce(&S) -> Option<PathBuf>,
{
    env::var_os(env_key)
        .map(PathBuf::from)
        .or_else(|| strategy_fn(strategy))
        .unwrap_or_else(|| env::temp_dir().join(constants::APP_NAME))
}


