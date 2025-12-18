 // hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod app;
mod constants;


use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1000.0, 750.0])
        .with_resizable(false) // Suits tiling window manager
        .with_decorations(false)
         // Wayland user can use app-id to customize window's behavior
        .with_app_id(constants::APP_ID)
        .with_drag_and_drop(true);

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        constants::APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
