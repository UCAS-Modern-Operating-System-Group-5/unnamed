use crate::ui;

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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui using custom fonts");
            ui.text_edit_multiline(&mut "你好\nEl Psy Congaroo!");
        });
    }
}


