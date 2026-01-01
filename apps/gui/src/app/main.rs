use crate::ui;
use crate::config::Config;
use crate::constants;
use crate::components::{self, ContextComponent};
use crate::backend;
use super::Scope;

#[derive(Default)]
pub struct App {
    config: Config,
    
    s: State,
    
    status_bar: components::StatusBar,
}

#[derive(Default)]
pub struct State {
    cur_scope: Scope,

    // Window State
    dropped_files: Vec<egui::DroppedFile>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    // Setup things like UI
    pub fn setup(&self, cc: &eframe::CreationContext<'_>) {
        ui::setup_ui(&cc.egui_ctx, &self.config.ui);
        self.setup_debug_options(&cc.egui_ctx);
    }

    fn setup_debug_options(&self, ctx: &egui::Context) {
        ctx.style_mut(|style| {
            style.debug.debug_on_hover_with_all_modifiers = true
        });
    }

    /// Should set search path to the parent directory of the file; Or if the dropped
    /// stuff is a directory, then set the search path to that directory
    pub fn handle_file_drop(&mut self, _ctx: &egui::Context) {
        // pass
    }

    pub fn update_window_title(&self, ctx: &egui::Context) {
        let title = constants::APP_NAME.to_string();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub fn render_status_bar(&mut self, ctx: &egui::Context) {
        let server_status = backend::ServerStatus::Online(
            backend::ServerWorkingStatus::Searching
        );
        let props = components::StatusBarProps {
            server_status
        };
        let status_bar_output = self.status_bar.render(ctx, props);

        for _event in status_bar_output.events {
            // TODO handle status bar events
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        let color = egui::lerp(
            egui::Rgba::from(visuals.panel_fill)..=egui::Rgba::from(visuals.extreme_bg_color),
            0.5,
        );
    
        let mut color = egui::Color32::from(color);
        color = color.gamma_multiply(self.config.app.background_alpha); 

        color.to_normalized_gamma_f32()
    }


    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }

        self.handle_file_drop(ctx);
        
        self.update_window_title(ctx);

        self.render_status_bar(ctx);


        // TODO no_frame() function in 0.33.4
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            ui.heading("egui using custom fonts");
            ui.text_edit_multiline(&mut "你好\nEl Psy Congaroo!");
            ui.label(format!("{}", ui.text_style_height(&egui::TextStyle::Body)));

            ui.horizontal(|ui| {
                let row_height = ui.text_style_height(&egui::TextStyle::Body);
                let (rect, response) = ui.allocate_exact_size([10.0, row_height].into(), egui::Sense::hover());
                ui.painter().circle_filled(
                    rect.center(),
                    rect.height() / 8.0,
                    ui.visuals().strong_text_color(),
                );
                ui.label(format!("{}", ui.text_style_height(&egui::TextStyle::Body)));
            });
        });
    }
}

