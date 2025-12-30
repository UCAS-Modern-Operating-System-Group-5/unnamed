use crate::ui;
use crate::config::Config;
use super::Scope;

#[derive(Default)]
pub struct App {
    config: Config,
    
    current_scope: Scope,
    
    dropped_files: Vec<egui::DroppedFile>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        ui::theme::setup_fonts(&cc.egui_ctx);
        Self {
            config,
            ..Default::default()
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::from_black_alpha(self.config.app.background_alpha).to_array()
        // egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }

        // TODO no_frame() function in 0.33.4
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            ui.heading("egui using custom fonts");
            ui.text_edit_multiline(&mut "你好\nEl Psy Congaroo!");
        });
    }
}

