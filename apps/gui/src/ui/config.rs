use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct UiConfig {
    /// Scale the whole UI
    pub scale: Option<f32>,
    pub font_size: f32,
}


impl Default for UiConfig {
    fn default() -> Self {
        Self {
            scale: None,
            font_size: 16.0,
        }
    }
}
