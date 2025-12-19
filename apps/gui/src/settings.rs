use crate::error::Result;
use crate::constants;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub window: WindowSettings
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WindowSettings {
    pub width: f32,
    pub height: f32
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1000.0,
            height: 750.0
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window: WindowSettings::default()
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
