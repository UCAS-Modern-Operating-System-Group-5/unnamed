use super::ContextComponent;
use crate::backend::ServerStatus;
use egui::{Response, Sense, TextStyle, Ui, Widget, pos2, vec2};

#[derive(Default)]
pub struct StatusBar {
    panel_height: f32,
}

pub struct StatusBarProps {
    pub server_status: ServerStatus,
}

/// Events emitted by the status bar
pub enum StatusBarEvent {
    /// User clicked on the restart button by the side of server status label
    RestartServer,
}

// TODO show restart server button when server status is `offline`
/// We make status bar status a widget since we want it to be able keep the
/// order of the status circle and the text in a right-to-left context (in `Sides`)
struct StatusBarStatusWidget {
    status: ServerStatus,
}

impl StatusBarStatusWidget {
    fn new(status: ServerStatus) -> Self {
        Self { status }
    }
}

impl Widget for StatusBarStatusWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let circle_radius = 6.0;
        let circle_diam = circle_radius * 2.0;
        let gap = ui.spacing().item_spacing.x;

        let text = format!("{}", self.status);
        let font_id = TextStyle::Body.resolve(ui.style());
        let text_color = ui.visuals().text_color();

        // Layout the text without drawing it yet
        let galley = ui.painter().layout_no_wrap(text, font_id, text_color);

        let width = circle_diam + gap + galley.size().x;
        let height = f32::max(circle_diam, galley.size().y);

        let desired_size = vec2(width, height);

        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let circle_center = pos2(rect.min.x + circle_radius, rect.center().y);
            // TODO
            ui.painter().circle_filled(
                circle_center,
                circle_radius,
                ui.style().visuals.error_fg_color
            );

            let text_pos = pos2(
                rect.min.x + circle_diam + gap,
                rect.center().y - (galley.size().y / 2.0),
            );

            ui.painter().galley(text_pos, galley, egui::Color32::PLACEHOLDER);
        }

        response
    }
}

impl StatusBar {
    pub fn height(&self) -> f32 {
        return self.panel_height;
    }
}

/// Output from status bar component
pub struct StatusBarOutput {
    pub events: Vec<StatusBarEvent>,
}

impl ContextComponent for StatusBar {
    type Props<'a> = StatusBarProps;
    type Output = StatusBarOutput;

    fn render(&mut self, ctx: &egui::Context, props: Self::Props<'_>) -> Self::Output {
        let mut events = Vec::new();

        let resp = egui::TopBottomPanel::bottom("status_bar")
            .show_separator_line(false)
            .frame(
                egui::Frame::NONE
                    .inner_margin(egui::vec2(4.0, 2.0))
                    .fill(ctx.style().visuals.extreme_bg_color)
            )
            .show(ctx, |ui| {
                egui::Sides::new().shrink_left().show(
                    ui,
                    |ui| {
                        ui.label("The Currently indexed file name");
                    },
                    |ui| {
                        ui.add(StatusBarStatusWidget::new(props.server_status));
                    },
                );
            });

        self.panel_height = resp.response.rect.size().y;

        StatusBarOutput { events }
    }
}
