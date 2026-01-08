use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use config::{create_strategy, resolve_dir, AppStrategy};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", default="default_config", deny_unknown_fields)]
pub struct Config {
    pub runtime_dir: PathBuf,
    pub cache_dir: PathBuf,
}


fn default_config() -> Config {
    let strategy = create_strategy().unwrap();

    Config {
        runtime_dir: resolve_dir("RUNTIME_DIRECTORY", &strategy, |s| {
            s.runtime_dir()
        }),
        cache_dir: resolve_dir("CACHE_DIRECTORY", &strategy, |s| {
            Some(s.cache_dir())
        }),
    }
}
    

impl Config {
    fn load_str(user_config_str: &str) -> Result<Config> {
        let user_config: Config = toml::from_str(user_config_str)?;
        Ok(user_config)
    }

    pub fn load() -> Result<Config> {
        let strategy = create_strategy()?;
        let config_path = strategy.config_dir().join(config::constants::SERVER_CONFIG_FILE_NAME);

        match std::fs::read_to_string(&config_path) {
            Ok(user_config_str) => Self::load_str(&user_config_str),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::load_str(""),
            Err(e) => Err(e.into()),
        }
    }
}
