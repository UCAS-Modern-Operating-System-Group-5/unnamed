use egui::Color32;
use super::{ColorPalette, Theme, hex};

struct Hexa34cDark;
impl ColorPalette for Hexa34cDark {
    fn is_dark(&self) -> bool { true }
    
    fn primary(&self) -> Color32 { hex!("#9ad4a1") }
    fn on_primary(&self) -> Color32 { hex!("#003916") }
    
    fn secondary(&self) -> Color32 { hex!("#b7ccb6") }
    fn on_secondary(&self) -> Color32 { hex!("#233425") }
    
    fn surface(&self) -> Color32 { hex!("#101510") }
    fn on_surface(&self) -> Color32 { hex!("#dfe4dc") }
    
    fn surface_variant(&self) -> Color32 { hex!("#101510") }
    fn on_surface_variant(&self) -> Color32 { hex!("#c1c9be") }
    
    fn error(&self) -> Color32 { hex!("#ffb4ab") }
    
    fn outline(&self) -> Color32 { hex!("#8b9389") }
    
    fn hover(&self) -> Color32 { hex!("#bcebc1") }
    fn on_hover(&self) -> Color32 { hex!("#003916") }
    
    fn shadow(&self) -> Color32 { hex!("#000000") }
}

struct Hexa34cLight;
impl ColorPalette for Hexa34cLight {
    fn is_dark(&self) -> bool { false }
    
    fn primary(&self) -> Color32 { hex!("#346940") }
    fn on_primary(&self) -> Color32 { hex!("#ffffff") }
    
    fn secondary(&self) -> Color32 { hex!("#506351") }
    fn on_secondary(&self) -> Color32 { hex!("#ffffff") }
    
    fn surface(&self) -> Color32 { hex!("#d7dbd3") }
    fn on_surface(&self) -> Color32 { hex!("#181d18") }
    
    fn surface_variant(&self) -> Color32 { hex!("#f6fbf2") }
    fn on_surface_variant(&self) -> Color32 { hex!("#414941") }
    
    fn error(&self) -> Color32 { hex!("#ba1a1a") }
    
    fn outline(&self) -> Color32 { hex!("#727970") }
    
    fn hover(&self) -> Color32 { hex!("#2a5433") }
    fn on_hover(&self) -> Color32 { hex!("#ffffff") }
    
    fn shadow(&self) -> Color32 { hex!("#000000") }
}

pub fn hexa34c_dark(alpha: f32) -> Theme {
    Theme::new("Hexa34c Dark", Hexa34cDark, alpha)
}

pub fn hexa34c_light(alpha: f32) -> Theme {
    Theme::new("Hexa34c Light", Hexa34cLight, alpha)
}
