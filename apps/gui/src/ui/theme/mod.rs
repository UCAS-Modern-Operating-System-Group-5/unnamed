use egui::{
    epaint::{Shadow, Stroke},
    style::{Selection, WidgetVisuals, Widgets, TextCursorStyle},
    Color32, Context, CornerRadius, Visuals,
};

pub trait ColorPalette {
    fn is_dark(&self) -> bool;

    fn primary(&self) -> Color32;
    fn on_primary(&self) -> Color32;

    fn secondary(&self) -> Color32;
    fn on_secondary(&self) -> Color32;

    fn surface(&self) -> Color32; // Main background
    fn on_surface(&self) -> Color32; // Main text

    fn surface_variant(&self) -> Color32; // Slightly different background (e.g. cards/panels)
    fn on_surface_variant(&self) -> Color32;

    fn error(&self) -> Color32;
    fn outline(&self) -> Color32; // Borders
    fn hover(&self) -> Color32;   // Hover state background
    fn on_hover(&self) -> Color32;
    fn shadow(&self) -> Color32;
}

pub struct Theme {
    pub name: String,
    pub visuals: Visuals,
}

impl Theme {
    /// Create a new theme with a specific transparency alpha.
    /// alpha: 0.0 (Invisible) to 1.0 (Opaque).
    pub fn new(name: &str, palette: impl ColorPalette, alpha: f32) -> Self {
        let common_rounding = CornerRadius::same(2);
        
        let surface_alpha = palette.surface().gamma_multiply(alpha);
        let surface_variant_alpha = palette.surface_variant().gamma_multiply(alpha);
        
        let visuals = Visuals {
            dark_mode: palette.is_dark(),
            override_text_color: Some(palette.on_surface()),
            // Apply alpha to main window/panel backgrounds
            window_fill: surface_alpha,
            panel_fill: surface_alpha,
            // Borders (Usually we keep borders opaque, or they look blurry)
            window_stroke: Stroke::new(1.0, palette.outline()),
            window_corner_radius: common_rounding,
            // Shadows
            window_shadow: Shadow {
                color: palette.shadow().gamma_multiply(0.5),
                offset: [8, 12],
                blur: 15,
                spread: 0,
            },
            popup_shadow: Shadow {
                color: palette.shadow().gamma_multiply(0.5),
                offset: [4, 8],
                blur: 8,
                spread: 0,
            },
            // Selection
            selection: Selection {
                bg_fill: palette.primary().gamma_multiply(0.4),
                stroke: Stroke::new(1.0, palette.primary()),
            },
            faint_bg_color: palette.on_surface().gamma_multiply(0.05),
            extreme_bg_color: surface_variant_alpha,
            text_edit_bg_color: Some(palette.surface_variant().gamma_multiply((alpha + 1.0) / 2.0)),
            text_cursor: TextCursorStyle {
                stroke: Stroke::new(2.0, palette.primary()),
                ..Default::default()
            },
            // Specific Colors
            hyperlink_color: palette.secondary(),
            warn_fg_color: palette.primary(),
            error_fg_color: palette.error(),
            code_bg_color: surface_variant_alpha,
            // WIDGET STYLES
            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    bg_fill: surface_alpha,
                    weak_bg_fill: surface_alpha,
                    bg_stroke: Stroke::new(1.0, palette.outline()),
                    fg_stroke: Stroke::new(1.0, palette.on_surface()),
                    corner_radius: common_rounding,
                    expansion: 0.0,
                },
                inactive: WidgetVisuals {
                    // Buttons/Inputs at rest
                    bg_fill: surface_variant_alpha,
                    weak_bg_fill: surface_variant_alpha,
                    bg_stroke: Stroke::new(1.0, palette.outline()),
                    fg_stroke: Stroke::new(1.0, palette.on_surface_variant()),
                    corner_radius: common_rounding,
                    expansion: 0.0,
                },
                hovered: WidgetVisuals {
                    // Hover states usually need to be more opaque to be seen clearly
                    bg_fill: palette.hover().gamma_multiply(0.9), 
                    weak_bg_fill: palette.hover().gamma_multiply(0.9),
                    bg_stroke: Stroke::new(1.0, palette.primary()),
                    fg_stroke: Stroke::new(1.0, palette.on_hover()),
                    corner_radius: common_rounding,
                    expansion: 1.0,
                },
                active: WidgetVisuals {
                    bg_fill: palette.primary(),
                    weak_bg_fill: palette.primary(),
                    bg_stroke: Stroke::new(1.0, palette.primary()),
                    fg_stroke: Stroke::new(1.0, palette.on_primary()),
                    corner_radius: common_rounding,
                    expansion: 1.0,
                },
                open: WidgetVisuals {
                    bg_fill: surface_variant_alpha,
                    weak_bg_fill: surface_variant_alpha,
                    bg_stroke: Stroke::new(1.0, palette.outline()),
                    fg_stroke: Stroke::new(1.0, palette.on_surface()),
                    corner_radius: common_rounding,
                    expansion: 0.0,
                },
            },
            ..if palette.is_dark() {
                Visuals::dark()
            } else {
                Visuals::light()
            }
        };
        Self {
            name: name.to_string(),
            visuals,
        }
    }
    pub fn apply(self, ctx: &Context) {
        ctx.set_visuals(self.visuals);
    }
}

macro_rules! hex {
    ($s:literal) => {{
        egui::Color32::from_hex($s).unwrap()
    }};
}

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

