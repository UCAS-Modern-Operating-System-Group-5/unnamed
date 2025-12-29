use crate::ui;
use crate::scope::Scope;
use serde::Deserialize;

#[derive(Default)]
pub struct App {
    current_scope: Scope,
    dropped_files: Vec<egui::DroppedFile>,
    
    background_alpha: f32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, app_config: &AppConfig) -> Self {
        ui::theme::setup_fonts(&cc.egui_ctx);
        Self {
            background_alpha: app_config.background_alpha,
            ..Self::default()
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from_black_alpha(self.background_alpha).to_array()
        // egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            ui.heading("egui using custom fonts");
            ui.text_edit_multiline(&mut "你好\nEl Psy Congaroo!");
        });
    }
}



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
            background_alpha: 1.0,
        }
    }
}

