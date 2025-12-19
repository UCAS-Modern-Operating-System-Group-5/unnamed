 // hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod app;
mod settings;
mod constants;
mod error;

use log::error;


use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();

    let config = match settings::Settings::from_file_or_env(None, constants::ENV_PREFIX) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load settings: {:#}", e);
            settings::Settings::default() 
        }
    };

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([config.window.width, config.window.height])
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
