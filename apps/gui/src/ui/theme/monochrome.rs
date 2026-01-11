use egui::Color32;
use super::{ColorPalette, Theme, hex};

struct MonochromeDark;
impl ColorPalette for MonochromeDark {
    fn is_dark(&self) -> bool { true }
    fn primary(&self) -> Color32 { hex!("#aaaaaa") }
    fn on_primary(&self) -> Color32 { hex!("#111111") }
    fn secondary(&self) -> Color32 { hex!("#a7a7a7") }
    fn on_secondary(&self) -> Color32 { hex!("#111111") }
    fn surface(&self) -> Color32 { hex!("#111111") }
    fn on_surface(&self) -> Color32 { hex!("#828282") }
    fn surface_variant(&self) -> Color32 { hex!("#191919") }
    fn on_surface_variant(&self) -> Color32 { hex!("#5d5d5d") }
    fn error(&self) -> Color32 { hex!("#dddddd") }
    fn outline(&self) -> Color32 { hex!("#3c3c3c") }
    fn hover(&self) -> Color32 { hex!("#cccccc") }
    fn on_hover(&self) -> Color32 { hex!("#111111") }
    fn shadow(&self) -> Color32 { hex!("#000000") }
}

struct MonochromeLight;
impl ColorPalette for MonochromeLight {
    fn is_dark(&self) -> bool { false }
    fn primary(&self) -> Color32 { hex!("#555555") }
    fn on_primary(&self) -> Color32 { hex!("#eeeeee") }
    fn secondary(&self) -> Color32 { hex!("#505058") }
    fn on_secondary(&self) -> Color32 { hex!("#eeeeee") }
    fn surface(&self) -> Color32 { hex!("#d4d4d4") }
    fn on_surface(&self) -> Color32 { hex!("#696969") }
    fn surface_variant(&self) -> Color32 { hex!("#e8e8e8") }
    fn on_surface_variant(&self) -> Color32 { hex!("#9e9e9e") }
    fn error(&self) -> Color32 { hex!("#222222") }
    fn outline(&self) -> Color32 { hex!("#c3c3c3") }
    fn hover(&self) -> Color32 { hex!("#333333") }
    fn on_hover(&self) -> Color32 { hex!("#eeeeee") }
    fn shadow(&self) -> Color32 { hex!("#fafafa") }
}


pub fn monochrome_dark(alpha: f32) -> Theme {
    Theme::new("Monochrome Dark", MonochromeDark, alpha)
}

pub fn monochrome_light(alpha: f32) -> Theme {
    Theme::new("Monochrome Light", MonochromeLight, alpha)
}

