 // hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod app;


use eframe::egui;
use egui::FontDefinitions;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "App Name",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
