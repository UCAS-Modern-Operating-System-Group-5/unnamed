use crate::constants;
use crate::error::Result;
use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    pub runtime_dir: PathBuf,
    pub cache_dir: PathBuf,
}

fn create_strategy() -> std::result::Result<impl AppStrategy, etcetera::HomeDirError> {
    choose_app_strategy(AppStrategyArgs {
        top_level_domain: constants::TOP_LEVEL_DOMAIN.to_string(),
        author: constants::AUTHOR.to_string(),
        app_name: constants::APP_NAME.to_string(),
    })
}

fn resolve_dir<S, F>(env_key: &str, strategy: Option<&S>, strategy_fn: F) -> PathBuf
where
    S: AppStrategy,
    F: FnOnce(&S) -> PathBuf,
{
    if let Ok(path) = env::var(env_key) {
        return PathBuf::from(path);
    }

    if let Some(s) = strategy {
        return strategy_fn(s).to_path_buf();
    }

    env::temp_dir().join(constants::APP_NAME)
}


impl Default for Config {
    fn default() -> Self {
        let strategy = create_strategy().ok();

        Self {
            runtime_dir: resolve_dir("RUNTIME_DIRECTORY", strategy.as_ref(), |s| {
                s.runtime_dir().expect("Cannot find runtime directory")
            }),
            cache_dir: resolve_dir("CACHE_DIRECTORY", strategy.as_ref(), |s| {
                s.cache_dir()
            }),
        }
    }
}

impl Config {
    fn load_str(user_config_str: &str) -> Result<Config> {
        let user_config: Config = toml::from_str(user_config_str)?;
        Ok(user_config)
    }

    pub fn load() -> Result<Config> {
        let strategy = create_strategy()?;
        let config_path = strategy.config_dir().join(constants::CONFIG_FILE_NAME);

        match std::fs::read_to_string(&config_path) {
            Ok(user_config_str) => Self::load_str(&user_config_str),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::load_str(""),
            Err(e) => Err(e.into()),
        }
    }
}
