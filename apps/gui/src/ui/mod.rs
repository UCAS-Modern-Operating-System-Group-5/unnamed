// TODO Make it configurable (read from config)

mod config;
mod font;
pub mod icon;
pub mod shape;
mod theme;

pub use config::UiConfig;
pub use font::setup_fonts;
pub use theme::*;

use crate::constants;
use egui::{FontId, TextStyle};
use std::collections::BTreeMap;

pub fn setup_ui(ctx: &egui::Context, cfg: &config::UiConfig, alpha: f32) {
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
        (TextStyle::Name("SearchBar".into()), FontId::proportional(font_size * 1.618)),
        (TextStyle::Name("SearchBarKeyHint".into()), FontId::proportional(font_size)),
        (TextStyle::Name("StatusBar".into()), FontId::proportional(font_size))
    ]
    .into();
    style.text_styles = text_styles;

    // Theme
    let theme = hexa34c_light(alpha);
    if theme.is_dark {
        ctx.set_theme(egui::Theme::Dark);
    } else {
        ctx.set_theme(egui::Theme::Light);
    }
    style.visuals = theme.visuals;

    style.spacing = theme.spacing;

    ctx.set_style(style);
}
