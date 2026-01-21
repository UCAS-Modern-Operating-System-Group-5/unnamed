use crate::constants;
use crate::util::SearchStatus;
use egui::{
    Color32, Painter, Pos2, Response, Sense, Shape, Stroke, TextStyle, Ui, Widget, pos2,
    vec2,
};
use rpc::search::SearchStatus as RpcSearchStatus;
use std::f32::consts::{FRAC_PI_2, TAU};

const SPINNER_SPEED: f64 = 1.2; // rotations per second
const SPINNER_DOT_COUNT: usize = 8;

pub struct StatusBarStatusWidget<'a> {
    pub server_online: bool,
    pub search_status: &'a SearchStatus,
}

/// Represents the visual state of the status display
struct StatusDisplay {
    text: String,
    prefix: StatusPrefix,
}

enum StatusPrefix {
    None,
    Spinner,
    Icon(StatusIcon),
}

#[derive(Clone, Copy)]
enum StatusIcon {
    Success,
    Error,
    Cancelled,
}


impl Widget for StatusBarStatusWidget<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let display = self.build_display();

        // Layout measurements
        let font_id = TextStyle::Name(constants::TEXT_STYLE_STATUS_BAR.into()).resolve(ui.style());
        let text_size = font_id.size;
        let icon_size = text_size * 0.8;
        let indicator_radius = icon_size / 2.0;
        let gap = ui.spacing().item_spacing.x;
        let text_color = ui.visuals().text_color();

        let galley =
            ui.painter()
                .layout_no_wrap(display.text.clone(), font_id, text_color);

        // Calculate dimensions
        let prefix_width = match display.prefix {
            StatusPrefix::None => 0.0,
            _ => icon_size + gap,
        };
        let width = indicator_radius * 2.0 + gap + prefix_width + galley.size().x;
        let height = galley.size().y.max(indicator_radius * 2.0).max(icon_size);

        let (rect, response) =
            ui.allocate_exact_size(vec2(width, height), Sense::hover());

        if ui.is_rect_visible(rect) {
            let mut cursor_x = rect.min.x;
            let center_y = rect.center().y;

            // 1. Draw server status indicator (small colored dot)
            let indicator_center = pos2(cursor_x + indicator_radius, center_y);
            let indicator_color = if self.server_online {
                Color32::from_rgb(76, 175, 80) // Material Green 500
            } else {
                Color32::from_rgb(244, 67, 54) // Material Red 500
            };
            ui.painter().circle_filled(
                indicator_center,
                indicator_radius,
                indicator_color,
            );
            cursor_x += indicator_radius * 2.0 + gap;

            // 2. Draw prefix (spinner or icon)
            let prefix_center = pos2(cursor_x + icon_size / 2.0, center_y);
            match display.prefix {
                StatusPrefix::Spinner => {
                    Self::draw_spinner(ui, prefix_center, icon_size / 2.0, text_color);
                    cursor_x += icon_size + gap;
                }
                StatusPrefix::Icon(icon) => {
                    Self::draw_icon(ui.painter(), prefix_center, icon_size, icon);
                    cursor_x += icon_size + gap;
                }
                StatusPrefix::None => {}
            }

            // 3. Draw status text
            let text_pos = pos2(cursor_x, center_y - galley.size().y / 2.0);
            ui.painter().galley(text_pos, galley, text_color);
        }

        // Request repaint while spinner is active
        if matches!(display.prefix, StatusPrefix::Spinner) {
            ui.ctx().request_repaint();
        }

        response
    }
}

impl StatusBarStatusWidget<'_> {
    fn build_display(&self) -> StatusDisplay {
        match self.search_status {
            SearchStatus::Idle => StatusDisplay {
                text: if self.server_online {
                    "Ready"
                } else {
                    "Offline"
                }
                .into(),
                prefix: StatusPrefix::None,
            },

            SearchStatus::Working(working) => match &working.status {
                None => StatusDisplay {
                    text: "Initializing...".into(),
                    prefix: StatusPrefix::Spinner,
                },
                Some(RpcSearchStatus::InProgress { found_so_far }) => StatusDisplay {
                    text: format!("Searching... ({} found)", found_so_far),
                    prefix: StatusPrefix::Spinner,
                },
                Some(RpcSearchStatus::Completed { total_count }) => StatusDisplay {
                    text: format!("{} results", total_count),
                    prefix: StatusPrefix::Icon(StatusIcon::Success),
                },
                Some(RpcSearchStatus::Cancelled) => StatusDisplay {
                    text: "Cancelled".into(),
                    prefix: StatusPrefix::Icon(StatusIcon::Cancelled),
                },
                Some(RpcSearchStatus::Failed(_)) => StatusDisplay {
                    text: "Search failed".into(),
                    prefix: StatusPrefix::Icon(StatusIcon::Error),
                },
            },

            SearchStatus::Failed(err) => StatusDisplay {
                text: format!("Error: {:?}", err),
                prefix: StatusPrefix::Icon(StatusIcon::Error),
            },
        }
    }

    /// Draws a rotating dot spinner
    fn draw_spinner(ui: &Ui, center: Pos2, radius: f32, color: Color32) {
        let time = ui.input(|i| i.time);
        let rotation = (time * SPINNER_SPEED * TAU as f64) as f32;

        let dot_radius = radius * 0.2;
        let orbit_radius = radius * 0.65;

        for i in 0..SPINNER_DOT_COUNT {
            // Calculate angle: start at top (-π/2), go clockwise
            let base_angle = (i as f32 / SPINNER_DOT_COUNT as f32) * TAU - FRAC_PI_2;
            let angle = base_angle - rotation;

            // Fade: first dot is brightest, last is most transparent
            let progress = i as f32 / SPINNER_DOT_COUNT as f32;
            let alpha = 1.0 - progress * 0.8;

            let dot_center = center + vec2(angle.cos(), angle.sin()) * orbit_radius;
            let dot_color = color.gamma_multiply(alpha);

            ui.painter()
                .circle_filled(dot_center, dot_radius, dot_color);
        }
    }

    /// Draws a status icon (checkmark, X, or cancelled symbol)
    fn draw_icon(painter: &Painter, center: Pos2, radius: f32, icon: StatusIcon) {
        let stroke_width = 1.8;

        match icon {
            StatusIcon::Success => {
                // Checkmark ✓
                let color = Color32::from_rgb(76, 175, 80);
                let r = radius * 0.7;
                let points = [
                    center + vec2(-r * 0.5, r * 0.0),
                    center + vec2(-r * 0.1, r * 0.4),
                    center + vec2(r * 0.6, -r * 0.4),
                ];
                painter.add(Shape::line(
                    points.to_vec(),
                    Stroke::new(stroke_width, color),
                ));
            }

            StatusIcon::Error => {
                // X mark ✗
                let color = Color32::from_rgb(244, 67, 54);
                let r = radius * 0.5;
                painter.line_segment(
                    [center + vec2(-r, -r), center + vec2(r, r)],
                    Stroke::new(stroke_width, color),
                );
                painter.line_segment(
                    [center + vec2(r, -r), center + vec2(-r, r)],
                    Stroke::new(stroke_width, color),
                );
            }

            StatusIcon::Cancelled => {
                // Prohibition sign ⊘
                let color = Color32::from_rgb(255, 152, 0); // Orange
                let r = radius * 0.55;
                painter.circle_stroke(center, r, Stroke::new(stroke_width * 0.8, color));
                let diag = r * 0.707; // cos(45°)
                painter.line_segment(
                    [center + vec2(-diag, diag), center + vec2(diag, -diag)],
                    Stroke::new(stroke_width * 0.8, color),
                );
            }
        }
    }
}
