// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc; // Much faster allocator, can give 20% speedups: https://github.com/emilk/egui/pull/7029

mod app;
mod backend;
mod components;
mod config;
mod constants;
mod error;
mod ui;

use tracing::error;

use eframe::egui;

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();

    if let Some(arg) = std::env::args().skip(1).next() {
        match arg.as_str() {
            "--profile" => {
                #[cfg(feature = "profile-with-puffin")]
                start_puffin_server();

                #[cfg(not(feature = "profile-with-puffin"))]
                panic!(
                    "Unknown argument: {arg} - you need to enable the 'puffin' feature to use this."
                );
            }

            _ => {
                panic!("Unknown argument: {arg}");
            }
        }
    }

    let cfg = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            error!("Load user configuration failed: {e}");
            config::Config::load_str("").expect("Load empty configuration failed!")
        }
    };

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([cfg.app.width, cfg.app.height])
        .with_resizable(false) // Suits tiling window manager
        // .with_decorations(false)
        // Wayland user can use app-id to customize window's behavior
        .with_app_id(constants::APP_ID)
        .with_transparent(true)
        .with_drag_and_drop(true);

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        ::config::constants::APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cfg, cc)))),
    )
}

#[cfg(feature = "profile-with-puffin")]
fn start_puffin_server() {
    puffin::set_scopes_on(true);

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            info!("To install puffin_viewer, run: cargo install puffin_viewer");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[expect(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            error!("Failed to start puffin server: {err}");
        }
    }
}
