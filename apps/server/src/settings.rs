use crate::error::Result;
use crate::constants;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub runtime_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        let runtime_base = env::var("RUNTIME_DIRECTORY")
            .or_else(|_| env::var("XDG_RUNTIME_DIR"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                env::temp_dir() 
            });

        let cache_base = env::var("CACHE_DIRECTORY")
            .or_else(|_| env::var("XDG_CACHE_HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                env::var("HOME")
                    .map(|home| PathBuf::from(home).join(".cache"))
                    .unwrap_or_else(|_| env::temp_dir())
            });
        
        Self {
            runtime_dir: runtime_base.join(constants::NAME), 
            cache_dir: cache_base.join(constants::NAME),
        }
    }
}

impl Settings {
    pub fn from_file_or_env(location: Option<&str>, env_prefix: &str) -> Result<Self> {
        let defaults = Self::default();
        let defaults_json = serde_json::to_string(&defaults)?;

        let mut builder = config::Config::builder()
            .add_source(config::File::from_str(
                &defaults_json, 
                config::FileFormat::Json5
            ));
        
        if let Some(location) = location {
            builder = builder.add_source(config::File::with_name(location).required(false));
        }
        
        let config = builder
            .add_source(
                config::Environment::with_prefix(env_prefix)
                    .prefix_separator("_")
                    .separator("__"),
            )
            .build()?;
            
        Ok(config.try_deserialize()?)
    }
}
