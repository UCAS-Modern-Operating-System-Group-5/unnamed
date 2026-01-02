use crate::{
    backend::{ServerStatus, ServerWorkingStatus},
    components::ContextComponent,
    ui::shape
};
use egui::Color32;
use crate::ui::get_cur_theme_extra_palette;

#[derive(Default)]
pub struct StatusBar;

pub struct StatusBarProps {
    pub server_status: ServerStatus,
}

#[derive(Default)]
pub struct StatusBarStatus;

/// Events emitted by the status bar
pub enum StatusBarEvent {
    /// User clicked on the restart button by the side of server status label
    RestartServer,
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
                ui.horizontal(|ui| {
                    shape::inline_filled_circle(ui, 6.0, palette.error);
                    shape::inline_filled_circle(ui, 6.0, palette.success);
                    shape::inline_filled_circle(ui, 6.0, palette.warning);
                    shape::inline_filled_circle(ui, 6.0, palette.info);
                    ui.label("El Psy Congraoo");
                })
            });

        StatusBarOutput { events }
    }
}
