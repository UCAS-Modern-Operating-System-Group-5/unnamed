use crate::app::AppConfig;
use crate::constants;
use crate::error::Result;
use crate::key::{KeyConfig, default_key_config, merge_key_config};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fmt::Display;
use std::io::Error as IOError;
use std::path::Path;
use toml::de::Error as TomlError;
use crate::user_command::UserCommand;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    pub app: AppConfig,
    pub keys: KeyConfig,
}


impl Config {
    pub fn load_str(user_config_str: &str,) -> Result<Config> {
        let user_config: Config = toml::from_str(user_config_str)?;

        let mut keys = default_key_config();
        merge_key_config(&mut keys, user_config.keys);

        let res = Self {
            app: user_config.app,
            keys
        };

        Ok(res)
    }

    pub fn load() -> Result<Config> {
        let strategy = choose_app_strategy(AppStrategyArgs {
            top_level_domain: constants::TOP_LEVEL_DOMAIN.to_string(),
            author: constants::AUTHOR.to_string(),
            app_name: constants::APP_NAME.to_string(),
        })?;

        let config_path = strategy.config_dir().join("config.toml");
        
        match std::fs::read_to_string(&config_path) {
            Ok(user_config_str) => Self::load_str(&user_config_str),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Self::load_str("") 
            }
            Err(e) => {
                Err(e.into()) 
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    
    use crate::user_command::UserCommand;
    use crate::scope::Scope;
    use crate::key::KeyShortcut;


    #[test]
    fn test_load_app_config() {
        const USER_CONFIG: &'static str = r#"
        [app]
        width = 200.0
        "#;
        let cfg = Config::load_str(USER_CONFIG).expect("Failed to load config");
        assert_eq!(cfg.app.width, 200.0);
        assert_eq!(cfg.app.height, 600.0);
    }

    #[test]
    fn test_key_config() {
        const USER_CONFIG: &'static str = r#"
        [keys.global]
        "C-Q" = "next-item"
        "#;

        let cfg = Config::load_str(USER_CONFIG).expect("Failed to load config");
        
        let target_key = KeyShortcut(egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL,
            egui::Key::Q
        ));

        let command = cfg.keys.get(&Scope::Global).and_then(|gkm| gkm.get(&target_key));
        assert_eq!(command, Some(&UserCommand::NextItem));
    }
}
