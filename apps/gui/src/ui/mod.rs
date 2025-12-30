// TODO Make it configurable (read from config)

mod theme;
mod font;
mod config;

pub use font::setup_fonts;
pub use config::UiConfig;
pub use theme::*;

use egui::{TextStyle, FontId};
use std::collections::BTreeMap;


pub fn setup_ui(ctx: &egui::Context, cfg: &config::UiConfig) {
    let mut style = (*ctx.style()).clone();
    
    setup_fonts(ctx);
    
    // UI Scale
    if let Some(scale) = cfg.scale {
        ctx.set_pixels_per_point(scale);
    }

    // Font Size
    let font_size = cfg.font_size;
    let text_styles: BTreeMap<_, _> = [
        (TextStyle::Small, FontId::proportional(font_size * 0.85)),
        (TextStyle::Body, FontId::proportional(font_size)),
        (TextStyle::Heading, FontId::proportional(font_size * 1.2)),
        (TextStyle::Monospace, FontId::monospace(font_size)),
        (TextStyle::Button, FontId::monospace(font_size)),
    ].into();
    style.text_styles = text_styles;

    // Theme
    style.visuals = modus_operandi().visuals;

    ctx.set_style(style);
}

