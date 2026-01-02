use crate::app::AppConfig;
use crate::ui::UiConfig;
use crate::constants;
use crate::error::Result;
use crate::app::{KeyConfig, default_key_config, merge_key_config};
use serde::Deserialize;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Config {
    pub app: AppConfig,
    pub ui: UiConfig,
    pub keys: KeyConfig,
}


impl Config {
    pub fn load_str(user_config_str: &str,) -> Result<Config> {
        let user_config: Config = toml::from_str(user_config_str)?;

        let mut keys = default_key_config();
        merge_key_config(&mut keys, user_config.keys);

        let res = Self {
            app: user_config.app,
            ui: user_config.ui,
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

        let config_path = strategy.config_dir().join(constants::CONFIG_FILE_NAME);
        
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
    
    use crate::app::UserCommand;
    use crate::app::Scope;
    use crate::app::KeyShortcut;


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
    fn test_load_app_config_unknown_field() {
        const USER_CONFIG: &'static str = r#"
        [app]
        Am-i-kawaii = "🥰"
        "#;
        let cfg = Config::load_str(USER_CONFIG);
        let err = cfg.unwrap_err();
        assert!(err.to_string().contains("unknown field `Am-i-kawaii`"));
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
