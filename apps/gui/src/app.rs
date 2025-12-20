use crate::ui;
use serde::Deserialize;

pub struct App {
    dropped_files: Vec<egui::DroppedFile>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ui::theme::setup_fonts(&cc.egui_ctx);
        Self {
            dropped_files: Default::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }


        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui using custom fonts");
            ui.text_edit_multiline(&mut "你好\nEl Psy Congaroo!");
        });
    }
}



#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct AppConfig {
    pub width: f32,
    pub height: f32
}


impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0
        }
    }
}

