mod modus_themes_palette;

use egui::{
    Visuals, Color32,
    style::{Widgets, WidgetVisuals, Selection, TextCursorStyle},
    epaint::{AlphaFromCoverage, Stroke, Shadow, CornerRadius}
};

pub struct Theme {
    pub name: String,
    pub visuals: Visuals
}

impl Theme {
    pub fn apply(self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.visuals = self.visuals;
        ctx.set_style(style);
    }
}

pub use modus_themes::*;

mod modus_themes {
    use super::*;

    /// A macro to generate a Modus theme function.
    ///
    /// # Arguments
    /// * `$func_name`: The name of the function to generate (e.g., modus_operandi).
    /// * `$theme_name`: The display name string (e.g., "Modus Operandi").
    /// * `$palette_mod`: The path to the palette module (e.g., modus_themes_palette::operandi).
    /// * `$is_dark`: Boolean literal for dark mode.
    macro_rules! define_modus_theme {
        ($func_name:ident, $theme_name:expr, $palette_mod:path, $is_dark:expr) => {
            #[allow(dead_code)]
            pub fn $func_name() -> Theme {
                // Import the specific palette constants into this scope
                use $palette_mod::*;

                // Base visuals depend on whether it's dark or light
                let bv = if $is_dark { Visuals::dark() } else { Visuals::light() };
                
                let modus_rounding = CornerRadius::same(2);

                Theme {
                    name: $theme_name.to_string(),
                    visuals: Visuals {
                        dark_mode: $is_dark,
                        // Use appropriate text rendering for the mode
                        text_alpha_from_coverage: if $is_dark { 
                            AlphaFromCoverage::DARK_MODE_DEFAULT 
                        } else { 
                            AlphaFromCoverage::LIGHT_MODE_DEFAULT 
                        },
                        
                        widgets: Widgets {
                            noninteractive: WidgetVisuals {
                                bg_fill: BG_MAIN,
                                weak_bg_fill: BG_MAIN,
                                bg_stroke: Stroke::new(1.0, BORDER),
                                fg_stroke: Stroke::new(1.0, FG_MAIN),
                                corner_radius: modus_rounding,
                                ..bv.widgets.noninteractive
                            },
                            inactive: WidgetVisuals {
                                bg_fill: BG_INACTIVE, 
                                weak_bg_fill: BG_INACTIVE,
                                bg_stroke: Stroke::new(1.0, BORDER), 
                                fg_stroke: Stroke::new(1.0, FG_MAIN),
                                corner_radius: modus_rounding,
                                ..bv.widgets.inactive
                            },
                            hovered: WidgetVisuals {
                                bg_fill: BG_HOVER,
                                weak_bg_fill: BG_HOVER,
                                bg_stroke: Stroke::new(1.0, FG_ALT),
                                fg_stroke: Stroke::new(1.5, FG_ALT),
                                corner_radius: modus_rounding,
                                ..bv.widgets.hovered
                            },
                            active: WidgetVisuals {
                                bg_fill: BG_ACTIVE,
                                weak_bg_fill: BG_ACTIVE,
                                bg_stroke: Stroke::new(1.0, BLUE_WARMER), 
                                fg_stroke: Stroke::new(2.0, FG_ALT),
                                corner_radius: modus_rounding,
                                ..bv.widgets.active
                            },
                            open: WidgetVisuals {
                                bg_fill: BG_DIM,
                                weak_bg_fill: BG_DIM,
                                bg_stroke: Stroke::new(1.0, BORDER),
                                fg_stroke: Stroke::new(1.0, FG_MAIN),
                                corner_radius: modus_rounding,
                                ..bv.widgets.open
                            },
                        },

                        selection: Selection {
                            bg_fill: BG_BLUE_SUBTLE,
                            stroke: Stroke { color: CYAN, width: 1.0 },
                        },

                        hyperlink_color: BLUE_WARMER,
                        faint_bg_color: BG_DIM,
                        extreme_bg_color: BG_MAIN,
                        text_edit_bg_color: Some(BG_MAIN), 
                        code_bg_color: BG_DIM,

                        warn_fg_color: YELLOW_WARMER, 
                        error_fg_color: RED,

                        window_fill: BG_MAIN,
                        window_stroke: Stroke { color: BORDER, width: 1.0 },
                        window_corner_radius: modus_rounding,
                        
                        window_shadow: Shadow {
                            color: Color32::from_black_alpha(40), 
                            offset: [8, 12],
                            blur: 15,
                            spread: 0,
                        },

                        panel_fill: BG_MAIN,
                        
                        popup_shadow: Shadow {
                            color: Color32::from_black_alpha(40),
                            ..bv.popup_shadow
                        },

                        text_cursor: TextCursorStyle {
                            stroke: Stroke::new(2.0, FG_MAIN),
                            ..bv.text_cursor
                        },

                        striped: true,
                        
                        ..bv
                    }
                }
            }
        };
    }

    define_modus_theme!(
        modus_operandi, 
        "Modus Operandi", 
        modus_themes_palette::operandi, 
        false
    );


    define_modus_theme!(
        modus_operandi_tinted, 
        "Modus Operandi Tinted", 
        modus_themes_palette::operandi_tinted, 
        false
    );


    define_modus_theme!(
        modus_vivendi, 
        "Modus Vivendi", 
        modus_themes_palette::vivendi, 
        true
    );

    define_modus_theme!(
        modus_vivendi_tinted, 
        "Modus Vivendi Tinted", 
        modus_themes_palette::vivendi_tinted, 
        true
    );

}
