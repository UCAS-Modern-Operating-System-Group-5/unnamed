use super::StatefulComponent;
use crate::app::UserCommand;
use crate::constants;
use crate::util::{
    SearchResultStore, SortConfig, SortMode, time::timestamp_to_local_string,
};
use egui_i18n::tr;
use rpc::search::{SearchHit, SearchMode};

#[derive(Default)]
pub struct SearchResultViewer {
    store: SearchResultStore,
    search_mode: SearchMode,
    // currently_selected
    display_mode: DisplayMode,
    preview_mode: PreviewMode,
    show_preview: bool,
}

#[derive(Default)]
enum DisplayMode {
    Comfortable,
    #[default]
    Compact,
}

#[derive(Default)]
enum PreviewMode {
    #[default]
    RightPreview,
    LeftPreview,
    NoPreview,
}

pub struct SearchResultViewerProps {}

pub struct SearchResultViewerOutput {
    pub events: Vec<SearchResultViewerEvent>,
}

pub enum SearchResultViewerEvent {}

impl SearchResultViewer {
    pub fn recieve_items(&mut self, items: impl IntoIterator<Item = SearchHit>) {
        self.store.extend(items);
    }

    pub fn clear_items(&mut self) {
        self.store.clear();
    }

    pub fn set_sort_config(&mut self, config: SortConfig) {
        self.store.set_sort_config(config);
    }

    pub fn set_search_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
        if matches!(self.search_mode, SearchMode::Natural)
            && matches!(self.store.sort_config().mode, SortMode::Score)
        {
            self.store.toggle_or_set_mode(SortMode::FilePath);
        }
    }

    pub fn handle_user_command(&self, cmd: &UserCommand) -> bool {
        match cmd {
            UserCommand::NextItem => true,
            UserCommand::PrevItem => true,
            _ => false,
        }
    }
}

pub struct SearchResultTable<'a> {
    store: &'a mut SearchResultStore,
    search_mode: &'a SearchMode,
    row_height: f32,

    events: Vec<SearchResultViewerEvent>,
}

impl egui_table::TableDelegate for SearchResultTable<'_> {
    fn header_cell_ui(
        &mut self,
        ui: &mut egui::Ui,
        cell_info: &egui_table::HeaderCellInfo,
    ) {
        let egui_table::HeaderCellInfo { col_range, .. } = cell_info;

        let sort_mode = &self.store.sort_config().mode;
        let time_col_index = match self.search_mode {
            SearchMode::Natural => 3,
            SearchMode::Rule => 2,
        };
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(4, 0))
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| match col_range.start {
                    0 => {
                        let resp = ui.heading(tr!("qrf-file-name")).on_hover_ui(|ui| {
                            ui.label(tr!("qrfd-file-name"));
                        });
                        if resp.clicked() {
                            dbg!("file-name");
                        }
                    }
                    1 => {
                        let resp = ui.heading(tr!("qrf-file-path")).on_hover_ui(|ui| {
                            ui.label(tr!("qrfd-file-path"));
                        });
                        if resp.clicked() {
                            dbg!("file-path");
                        }
                    }
                    val if val == 2
                        && matches!(self.search_mode, SearchMode::Natural) =>
                    {
                        ui.heading(tr!("qrf-score")).on_hover_ui(|ui| {
                            ui.label(tr!("qrfd-score"));
                        });
                    }
                    val if val == time_col_index => match sort_mode {
                        SortMode::AccessedTime => {
                            ui.heading(tr!("qrf-atime")).on_hover_ui(|ui| {
                                ui.label(tr!("qrfd-atime"));
                            });
                        }
                        SortMode::CreatedTime => {
                            ui.heading(tr!("qrf-ctime")).on_hover_ui(|ui| {
                                ui.label(tr!("qrfd-ctime"));
                            });
                        }
                        _ => {
                            ui.heading(tr!("qrf-mtime")).on_hover_ui(|ui| {
                                ui.label(tr!("qrfd-mtime"));
                            });
                        }
                    },
                    _ => {}
                });
            });
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell_info: &egui_table::CellInfo) {
        let egui_table::CellInfo { row_nr, col_nr, .. } = cell_info;

        if row_nr % 2 == 1 {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().faint_bg_color);
        }

        let sort_mode = self.store.sort_config().mode.clone();
        let item = self.store.get_sorted(*row_nr as usize);
        let time_col_index = match self.search_mode {
            SearchMode::Natural => 3,
            SearchMode::Rule => 2,
        };

        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(4, 0))
            .show(ui, |ui| {
                if let Some(hit) = item {
                    let file_name = hit
                        .file_path
                        .file_name()
                        .map(|v| v.to_string_lossy())
                        .unwrap_or("".into());
                    let file_path = hit
                        .file_path
                        .parent()
                        .map(|p| p.to_string_lossy())
                        .unwrap_or(hit.file_path.to_string_lossy());
                    match col_nr {
                        0 => {
                            ui.label(file_name);
                        }
                        1 => {
                            ui.label(file_path);
                        }
                        val if val == &2
                            && matches!(self.search_mode, SearchMode::Natural) =>
                        {
                            if let Some(score) = hit.score {
                                ui.label(format!("{}", score));
                            }
                        }
                        val if val == &time_col_index => match sort_mode {
                            SortMode::AccessedTime => {
                                ui.label(timestamp_to_local_string(
                                    hit.access_time as i64,
                                ));
                            }
                            SortMode::CreatedTime => {
                                ui.label(timestamp_to_local_string(
                                    hit.access_time as i64,
                                ));
                            }
                            _ => {
                                ui.label(timestamp_to_local_string(
                                    hit.access_time as i64,
                                ));
                            }
                        },
                        _ => {}
                    }
                }
            });
    }

    fn row_ui(&mut self, ui: &mut egui::Ui, _row_nr: u64) {
        if ui.rect_contains_pointer(ui.max_rect()) {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, ui.visuals().code_bg_color);
        }
    }

    fn row_top_offset(
        &self,
        ctx: &egui::Context,
        _table_id: egui::Id,
        row_nr: u64,
    ) -> f32 {
        row_nr as f32 * self.row_height
    }
}

// TODO Don't display result when the store is empty, show an Text or indicator
// TODO preview panel
impl StatefulComponent for SearchResultViewer {
    type Props<'a> = SearchResultViewerProps;
    type Output = SearchResultViewerOutput;

    fn render(&mut self, ui: &mut egui::Ui, props: Self::Props<'_>) -> Self::Output {
        let events = Vec::new();

        // TODO set `style.animation` to disable the scroll animation

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        let total_rows = self.store.len();

        let mut search_result_table = SearchResultTable {
            search_mode: &self.search_mode,
            store: &mut self.store,
            row_height,
            events: Default::default(),
        };

        let max_width = ui.available_width();
        let column_num = match search_result_table.search_mode {
            SearchMode::Natural => 4,
            SearchMode::Rule => 3,
        };
        let default_column_width = max_width / (column_num + 1) as f32;

        let default_column = egui_table::Column::new(default_column_width)
            .range(10.0..=max_width * 0.8)
            .resizable(true);

        let file_path_column = egui_table::Column::new(default_column_width * 2.0)
            .range(10.0..=max_width * 0.8)
            .resizable(true);

        let columns = match search_result_table.search_mode {
            SearchMode::Natural => vec![
                default_column,   // file name
                file_path_column, // file path
                default_column,   // score
                default_column,   // time
            ],
            SearchMode::Rule => vec![
                default_column,   // file name
                file_path_column, // file path
                default_column,   // time
            ],
        };

        let egui_table = egui_table::Table::new()
            .id_salt(constants::ID_SALT_SEARCH_RESULT_VIEWER_EGUI_TABLE)
            .columns(columns)
            .num_rows(total_rows as u64)
            .headers([egui_table::HeaderRow::new(row_height)])
            .auto_size_mode(egui_table::AutoSizeMode::Always);

        egui_table.show(ui, &mut search_result_table);

        let side_panel = match self.preview_mode {
            PreviewMode::RightPreview => Some(egui::SidePanel::right(
                constants::ID_PANEL_SEARCH_RESULT_VIEWER,
            )),
            PreviewMode::LeftPreview => Some(egui::SidePanel::left(
                constants::ID_PANEL_SEARCH_RESULT_VIEWER,
            )),
            PreviewMode::NoPreview => None,
        };

        if let Some(side_panel) = side_panel {
            let max_width = ui.available_width();
            side_panel
                .frame(
                    egui::Frame::NONE
                        .inner_margin(egui::vec2(4.0, 2.0))
                        // .fill(egui::Color32::TRANSPARENT),
                        .fill(ui.visuals().panel_fill.to_opaque().gamma_multiply(0.8)),
                )
                .resizable(true)
                .default_width(max_width * 0.38) // Golden ratio
                .min_width(max_width * 0.3)
                .max_width(max_width * 0.5)
                .show_animated(ui.ctx(), self.show_preview, |ui| {
                    ui.take_available_space();
                    ui.label("Preview");
                });
        }

        SearchResultViewerOutput { events }
    }
}
