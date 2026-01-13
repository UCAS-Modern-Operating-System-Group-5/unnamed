mod hexa34c;
mod monochrome;

pub use hexa34c::{hexa34c_dark, hexa34c_light};
pub use monochrome::{monochrome_dark, monochrome_light};

use egui::{
    Color32, Context, CornerRadius, Visuals,
    epaint::{Shadow, Stroke},
    style::{Selection, TextCursorStyle, WidgetVisuals, Widgets},
};

pub trait ColorPalette {
    fn is_dark(&self) -> bool;

    fn primary(&self) -> Color32;
    fn on_primary(&self) -> Color32;

    fn secondary(&self) -> Color32;
    #[allow(dead_code)]
    fn on_secondary(&self) -> Color32;

    fn surface(&self) -> Color32; // Main background
    fn on_surface(&self) -> Color32; // Main text

    fn surface_variant(&self) -> Color32; // Slightly different background (e.g. cards/panels)
    fn on_surface_variant(&self) -> Color32;

    fn error(&self) -> Color32;
    fn outline(&self) -> Color32; // Borders
    fn hover(&self) -> Color32; // Hover state background
    fn on_hover(&self) -> Color32;
    fn shadow(&self) -> Color32;
}

pub struct Theme {
    #[allow(dead_code)]
    pub name: String,
    pub is_dark: bool,
    pub visuals: Visuals,
}

impl Theme {
    /// Create a new theme with a specific transparency alpha.
    /// alpha: 0.0 (Invisible) to 1.0 (Opaque).
    pub fn new(
        name: &str,
        palette: impl ColorPalette,
        alpha: f32,
    ) -> Self {
        let common_rounding = CornerRadius::same(2);

        let surface_alpha = palette.surface().gamma_multiply(alpha);
        let surface_variant_alpha = palette.surface_variant().gamma_multiply(alpha);

        let visuals = Visuals {
            dark_mode: palette.is_dark(),
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
            text_edit_bg_color: Some(
                palette
                    .surface_variant()
                    .gamma_multiply((alpha + 1.0) / 2.0),
            ),
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
                    weak_bg_fill: palette.hover().gamma_multiply(0.5),
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
            is_dark: palette.is_dark() ,
            visuals,
        }
    }

    #[allow(dead_code)]
    pub fn apply(self, ctx: &Context) {
        ctx.set_visuals(self.visuals);
    }
}

macro_rules! hex {
    ($s:literal) => {{ egui::Color32::from_hex($s).unwrap() }};
}
pub(crate) use hex;
