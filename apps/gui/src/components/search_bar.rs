use super::ContextComponent;
use rpc::search::SearchMode;

use egui_i18n::tr;

#[derive(Default)]
pub struct SearchBar {
    raw_search_query: String,
    panel_height: f32,
    focus: bool,
}

#[derive(Default)]
pub struct SearchBarProps {
    pub search_mode: SearchMode,
    pub draw_separate_line: bool,
}

pub struct SearchBarOutput {
    pub events: Vec<SearchBarEvent>,
}

pub enum SearchBarEvent {
    /// Start search with raw query string
    StartSearch(String),
}

impl SearchBar {
    pub fn height(&self) -> f32 {
        return self.panel_height;
    }

    pub fn request_focus(&mut self) {
        self.focus = true;
    }
}

fn setup_text_edit_style(style: &mut egui::Style) {
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.text_cursor.stroke = egui::Stroke::new(4.0, style.visuals.text_color());

    // Note, focused text edit's border uses the same stroke
    // as selection, but we cannot directory set selection's stroke
    // to NONE otherwise the text selection won't work/appear
    // correctly. Currently we use our patched egui crate to
    // solve this problem.
    // style.visuals.selection.stroke = egui::Stroke::NONE;
}

impl ContextComponent for SearchBar {
    type Props<'a> = SearchBarProps;

    type Output = SearchBarOutput;

    fn render(&mut self, ctx: &egui::Context, props: Self::Props<'_>) -> Self::Output {
        let events = vec![];

        let raw_search_query = &mut self.raw_search_query;

        let resp = egui::TopBottomPanel::top("search_bar")
            .show_separator_line(props.draw_separate_line)
            .frame(
                egui::Frame::NONE
                    .inner_margin(egui::vec2(10.0, 6.0))
                    .fill(ctx.style().visuals.panel_fill),
            )
            .show(ctx, |ui| {
                let hint_text = match props.search_mode {
                    SearchMode::Natural => tr!("search-bar-natural-mode-hint"),
                    SearchMode::Rule => tr!("search-bar-rule-mode-hint"),
                };

                // dbg!(ui.style().visuals.weak_text_color());
                // dbg!(ui.style().visuals.text_color());

                ui.scope(|ui| {
                    let style = ui.style_mut();
                    setup_text_edit_style(style);

                    let resp = egui::TextEdit::singleline(raw_search_query)
                        .desired_width(f32::INFINITY)
                        .font(
                            egui::TextStyle::Name("SearchBar".into()).resolve(ui.style()),
                        )
                        .background_color(egui::Color32::TRANSPARENT)
                        .hint_text(hint_text)
                        .show(ui);

                    if self.focus {
                        resp.response.request_focus();
                        self.focus = false;
                    }
                });
            });

        self.panel_height = resp.response.rect.size().y;

        SearchBarOutput { events }
    }
}
