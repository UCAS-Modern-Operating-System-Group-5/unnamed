mod status_widget;

use super::ContextComponent;
use crate::ui::icon::icon_image;
use egui::{TextStyle, Widget};
use egui_i18n::tr;
use rpc::search::SearchMode;
use strum::{EnumCount, IntoEnumIterator};
use crate::constants;
use crate::util::{SortConfig, SortMode, SearchStatus};
use status_widget::StatusBarStatusWidget;

pub struct StatusBar {
    panel_height: f32,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self { panel_height: 0.0 }
    }
}

pub struct StatusBarProps<'a> {
    pub server_online: bool,
    pub search_status: &'a SearchStatus,
    pub search_mode: &'a SearchMode,
    pub sort_config: &'a SortConfig,
}

/// Events emitted by the status bar
pub enum StatusBarEvent {
    // /// User clicked on the restart button by the side of server status label
    // RestartServer,

    ChangeSortConfig(SortConfig),
    ChangeSearchMode(SearchMode),
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

fn set_borderless_button_style(style: &mut egui::Style) {
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
}

impl StatusBar {
    // TODO also render sort direction
    fn render_sort_mode_selector(
        &mut self,
        ui: &mut egui::Ui,
        sort_config: &SortConfig,
    ) -> Option<StatusBarEvent> {
        // Note, we don't use combobox since it cannot vertically centered in the context
        // where we use a different(larger) text style than TextStyle::Body
        // https://github.com/emilk/egui/issues/7412
        // (May not correct) Though the center position of menu_button changes(so the wight id
        // changes). The change is not frequent, so the performance cost is bearable.
        let sort_mode = sort_config.mode.clone();
        let mut selected = sort_mode.clone();
        let label_text = format!("S:{}", tr!(&sort_mode.to_string()));

        let style = ui.style_mut();
        set_borderless_button_style(style);

        ui.menu_button(label_text, |ui| {
            let style = ui.style_mut();
            style.override_text_style = Some(TextStyle::Name(constants::TEXT_STYLE_STATUS_BAR.into()));

            let menu_margin = egui::Frame::menu(style).total_margin();
            let max_available_height =
                ui.ctx().content_rect().height() - menu_margin.top - menu_margin.bottom;

            let mut hidden = ui.new_child(egui::UiBuilder::new().invisible());
            hidden.selectable_value(
                &mut selected,
                SortMode::AccessedTime,
                tr!(&SortMode::AccessedTime.to_string()),
            );
            let single_entry_height = hidden.min_rect().height();

            let row_num =
                ((max_available_height / single_entry_height).floor() as usize).max(1);
            let column_num = SortMode::COUNT.div_ceil(row_num);

            egui::Grid::new("sort_mode_selector")
                .num_columns(column_num)
                .show(ui, |ui| {
                    let modes = SortMode::iter().rev();

                    for (i, mode) in modes.enumerate() {
                        let is_selected = selected == mode;
                        let label_text = tr!(&mode.to_string());

                        let mut response =
                            egui::Button::new(label_text).selected(is_selected).ui(ui);

                        if response.clicked() && selected != mode {
                            selected = mode.clone();
                            response.mark_changed();
                        }

                        if (i + 1) % column_num == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        if selected != sort_mode {
            // 切换模式时，使用新模式的默认方向
            let mut new_config = sort_config.clone();
            new_config.toggle_or_set(selected);
            return Some(StatusBarEvent::ChangeSortConfig(new_config));
        }

        None
    }
}

fn render_search_mode_button(
    ui: &mut egui::Ui,
    search_mode: &SearchMode,
) -> Option<StatusBarEvent> {
    let (image, hint) = match search_mode {
        SearchMode::Natural => (
            icon_image!("sparkles.svg", None),
            tr!("search-mode-toggle-button-switch-to-rule-mode-hint"),
        ),
        SearchMode::Rule => (
            icon_image!("sliders-horizontal.svg", None),
            tr!("search-mode-toggle-button-switch-to-natural-mode-hint"),
        ),
    };

    ui.scope(|ui| {
        let style = ui.style_mut();
        set_borderless_button_style(style);

        if ui.button(image).on_hover_text(hint).clicked() {
            let next_search_mode = SearchMode::iter()
                .cycle()
                .skip_while(|m| m != search_mode)
                .skip(1)
                .next()
                .unwrap();

            return Some(StatusBarEvent::ChangeSearchMode(next_search_mode));
        }
        None
    })
    .inner
}

impl ContextComponent for StatusBar {
    type Props<'a> = StatusBarProps<'a>;
    type Output = StatusBarOutput;

    fn render(&mut self, ctx: &egui::Context, props: Self::Props<'_>) -> Self::Output {
        let mut events = Vec::new();

        let resp = egui::TopBottomPanel::bottom(constants::ID_PANEL_STATUS_BAR)
            .show_separator_line(false)
            .frame(
                egui::Frame::NONE
                    .inner_margin(egui::vec2(4.0, 2.0))
                    .fill(ctx.style().visuals.extreme_bg_color),
            )
            .show(ctx, |ui| {
                // Here we don't use `Sides` container since it creates two child UI
                // for left and right, causing icon and text doesn't appear to be the
                // same size.
                // Another way to handle this issue is to use sizing_pass and add space
                // https://github.com/emilk/egui/discussions/2916#discussioncomment-14723556
                let style = ui.style_mut();
                style.override_text_style = Some(TextStyle::Name(constants::TEXT_STYLE_STATUS_BAR.into()));

                ui.horizontal(|ui| {
                    ui.add(StatusBarStatusWidget {
                        server_online: props.server_online,
                        search_status: props.search_status,
                    });
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if let Some(event) =
                                render_search_mode_button(ui, props.search_mode)
                            {
                                events.push(event);
                            }

                            ui.add(egui::Separator::default().vertical().shrink(2.0));

                            if let Some(event) =
                                self.render_sort_mode_selector(ui, props.sort_config)
                            {
                                events.push(event);
                            }
                        },
                    );
                });
            });

        self.panel_height = resp.response.rect.size().y;

        StatusBarOutput { events }
    }
}
