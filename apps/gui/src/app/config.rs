use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct AppConfig {
    pub width: f32,
    pub height: f32,
    pub background_alpha: f32,
}


impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            background_alpha: 0.9,
        }
    }
}
