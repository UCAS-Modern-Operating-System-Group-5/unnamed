use crate::{backend::{ServerStatus, ServerWorkingStatus}, components::ContextComponent};

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
    RestartServer
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

        egui::TopBottomPanel::bottom("status_bar")
            // .exact_height(30.0)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("El Psy Congraoo");
                })
            });

        StatusBarOutput { events }
    }
}



