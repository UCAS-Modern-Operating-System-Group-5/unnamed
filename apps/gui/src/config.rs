use crate::app::AppConfig;
use crate::app::{KeyConfig, default_key_config, merge_key_config};
use crate::error::Result;
use crate::ui::UiConfig;
use serde::Deserialize;
use std::path::PathBuf;
use config::{AppStrategy, constants as config_constants, create_strategy, resolve_dir};

#[derive(Debug, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub ui: UiConfig,
    pub keys: KeyConfig,

    // === System state ===
    pub runtime_dir: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
struct RawConfig {
    app: AppConfig,
    ui: UiConfig,
    keys: KeyConfig,
}
impl Default for RawConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            ui: UiConfig::default(),
            keys: default_key_config(),
        }
    }
}

impl Config {
    fn from_raw(raw: RawConfig, runtime_dir: PathBuf, config_path: PathBuf) -> Self {
        let mut keys = default_key_config();
        merge_key_config(&mut keys, raw.keys);

        Self {
            app: raw.app,
            ui: raw.ui,
            keys,
            runtime_dir,
            config_path,
        }
    }

    pub fn load() -> Result<Config> {
        let strategy = create_strategy().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "Home dir not found")
        })?;

        let config_path =
            resolve_dir("CONFIG_DIRECTORY", &strategy, |s| Some(s.config_dir()))
                .join(config_constants::GUI_CONFIG_FILE_NAME);

        let runtime_dir =
            resolve_dir("RUNTIME_DIRECTORY", &strategy, |s| s.runtime_dir());
        
        let raw_config: RawConfig = match std::fs::read_to_string(&config_path) {
            Ok(content) => toml::from_str(&content)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => RawConfig::default(),
            Err(e) => return Err(e.into()),
        };

        Ok(Self::from_raw(raw_config, runtime_dir, config_path))
    }

    pub fn load_str(config_str: &str) -> Result<Self> {
        let raw: RawConfig = toml::from_str(config_str)?;
        
        let strategy = create_strategy().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "Home dir not found")
        })?;

        let config_path =
            resolve_dir("CONFIG_DIRECTORY", &strategy, |s| Some(s.config_dir()))
                .join(config_constants::GUI_CONFIG_FILE_NAME);

        let runtime_dir =
            resolve_dir("RUNTIME_DIRECTORY", &strategy, |s| s.runtime_dir());

        Ok(Self::from_raw(raw, runtime_dir, config_path))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::{KeyShortcut, Scope, UserCommand};

    #[test]
    fn test_app_config_defaults() {
        let default_app = AppConfig::default();
        assert_eq!(default_app.width, 800.0);
        assert_eq!(default_app.height, 600.0);
    }

    #[test]
    fn test_load_config_values() {
        const USER_CONFIG: &'static str = r#"
        [app]
        width = 200.0
        "#;

        let cfg = Config::load_str(USER_CONFIG).expect("Failed to load config");

        assert_eq!(cfg.app.width, 200.0);
        assert_eq!(cfg.app.height, 600.0);
    }

    #[test]
    fn test_load_config_unknown_field() {
        const USER_CONFIG: &'static str = r#"
        [app]
        Am-i-kawaii = "🥰"
        "#;

        let cfg = Config::load_str(USER_CONFIG);
        let err = cfg.unwrap_err();
        assert!(err.to_string().contains("unknown field `Am-i-kawaii`"));
    }

    #[test]
    fn test_key_config_merging() {
        const USER_CONFIG: &'static str = r#"
        [keys.global]
        "C-Q" = "next-item"
        "#;

        let cfg = Config::load_str(USER_CONFIG).expect("Failed to load config");

        let target_key = KeyShortcut(egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL,
            egui::Key::Q,
        ));

        let command = cfg
            .keys
            .get(&Scope::Global)
            .and_then(|gkm| gkm.get(&target_key));
        assert_eq!(command, Some(&UserCommand::NextItem));
    }
}
