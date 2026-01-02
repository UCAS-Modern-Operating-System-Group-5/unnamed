use crate::ui::get_cur_theme_extra_palette;
use crate::{
    backend::ServerStatus,
    components::ContextComponent,
    ui::{ThemeExtraPalette},
};
use egui::{Response, Ui, Widget, Sense, vec2, pos2, TextStyle};

#[derive(Default)]
pub struct StatusBar;

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
    palette: ThemeExtraPalette,
}

impl StatusBarStatusWidget {
    fn new(status: ServerStatus, palette: ThemeExtraPalette) -> Self {
        Self { status, palette }
    }
}

impl Widget for StatusBarStatusWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let circle_radius = 6.0;
        let circle_diam = circle_radius * 2.0;
        let gap = ui.spacing().item_spacing.x;

        let text = format!("{}", self.status);
        let font_id = ui.style().text_styles[&TextStyle::Body].clone();
        let text_color = ui.visuals().text_color();
        
        // Layout the text without drawing it yet
        let galley = ui.painter().layout_no_wrap(text, font_id, text_color);

        let width = circle_diam + gap + galley.size().x;
        let height = f32::max(circle_diam, galley.size().y);
        
        let desired_size = vec2(width, height);

        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let circle_center = pos2(
                rect.min.x + circle_radius, 
                rect.center().y
            );
            ui.painter().circle_filled(circle_center, circle_radius, self.palette.error);

            let text_pos = pos2(
                rect.min.x + circle_diam + gap,
                rect.center().y - (galley.size().y / 2.0)
            );
            
            ui.painter().galley(text_pos, galley, egui::Color32::PLACEHOLDER);
        }

        response
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

        let palette = get_cur_theme_extra_palette(ctx);

        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                egui::Sides::new().shrink_left().show(
                    ui,
                    |ui| {
                        ui.label("The Currently indexed file name");
                    },
                    |ui| {
                        ui.add(StatusBarStatusWidget::new(
                            props.server_status.clone(),
                            palette.clone()
                        ));
                    },
                );
            });

        StatusBarOutput { events }
    }
}
