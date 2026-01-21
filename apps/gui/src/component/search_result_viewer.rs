use super::StatefulComponent;
use crate::app::UserCommand;
use crate::constants;
use crate::ui::icon::file_icon_from_path;
use crate::util::{
    SearchResultStore, SortConfig, SortMode, time::timestamp_to_local_string,
};
use egui_i18n::tr;
use rpc::search::{SearchHit, SearchMode};
use std::cell::Cell;

#[derive(Default)]
pub struct SearchResultViewer {
    store: SearchResultStore,
    search_mode: SearchMode,
    selected_index: Option<usize>,
    show_preview: bool,
}

pub struct SearchResultViewerProps {}

pub struct SearchResultViewerOutput {
    pub events: Vec<SearchResultViewerEvent>,
}

pub enum SearchResultViewerEvent {
    FileSelected(std::path::PathBuf),
}

impl SearchResultViewer {
    pub fn recieve_items(&mut self, items: impl IntoIterator<Item = SearchHit>) {
        self.store.extend(items);
    }

    pub fn clear_items(&mut self) {
        self.store.clear();
        self.selected_index = None;
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

/// Result of rendering a search result card
struct CardRenderResult {
    response: egui::Response,
    file_name_rect: egui::Rect,
    file_path: std::path::PathBuf,
}

/// Render a single search result card
fn render_result_card(
    ui: &mut egui::Ui,
    hit: &SearchHit,
    _index: usize,
    is_selected: bool,
    search_mode: &SearchMode,
) -> CardRenderResult {
    let file_name = hit
        .file_path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    
    let file_path_display = hit
        .file_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| hit.file_path.to_string_lossy().to_string());

    // Card frame with selection styling
    let card_frame = if is_selected {
        egui::Frame::NONE
            .fill(ui.visuals().selection.bg_fill)
            .stroke(ui.visuals().selection.stroke)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .outer_margin(egui::Margin::symmetric(0, 2))
    } else {
        egui::Frame::NONE
            .fill(ui.visuals().extreme_bg_color)
            .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .outer_margin(egui::Margin::symmetric(0, 2))
    };

    // Store the file name label rect for later click detection
    let file_name_rect = Cell::new(egui::Rect::NOTHING);
    
    let resp = card_frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            
            // File info section
            ui.vertical(|ui| {
                // First row: file type icon + file name with score on the right
                ui.horizontal(|ui| {
                    // File type icon inline with name
                    let icon_size = 24.0;
                    let icon = file_icon_from_path(&hit.file_path, Some(icon_size));
                    ui.add(icon);
                    
                    ui.add_space(4.0);
                    
                    // File name - just display it with hyperlink styling
                    // We'll detect clicks separately after the frame
                    let text = egui::RichText::new(&file_name)
                        .strong()
                        .size(14.0)
                        .color(ui.visuals().hyperlink_color)
                        .underline();
                    
                    let label_response = ui.label(text);
                    file_name_rect.set(label_response.rect);
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Score (if available, for natural search)
                        if let Some(score) = hit.score {
                            if matches!(search_mode, SearchMode::Natural) {
                                let score_text = format!("{:.2}", score);
                                ui.label(
                                    egui::RichText::new(score_text)
                                        .small()
                                        .color(ui.visuals().warn_fg_color)
                                );
                            }
                        }
                    });
                });
                
                ui.add_space(2.0);
                
                // Second row: file path (muted color)
                ui.label(
                    egui::RichText::new(&file_path_display)
                        .small()
                        .color(ui.visuals().weak_text_color())
                );
                
                ui.add_space(2.0);
                
                // Third row: metadata (file size, modified time)
                ui.horizontal(|ui| {
                    // File size
                    let size_text = format_file_size(hit.file_size);
                    ui.label(
                        egui::RichText::new(&size_text)
                            .small()
                            .color(ui.visuals().weak_text_color())
                    );
                    
                    ui.add_space(16.0);
                    
                    // Modified time
                    let time_text = timestamp_to_local_string(hit.modified_time as i64);
                    ui.label(
                        egui::RichText::new(format!("修改于 {}", time_text))
                            .small()
                            .color(ui.visuals().weak_text_color())
                    );
                });
            });
        })
        .response;

    CardRenderResult {
        response: resp.interact(egui::Sense::click()),
        file_name_rect: file_name_rect.get(),
        file_path: hit.file_path.clone(),
    }
}

/// Format file size in human-readable format
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

impl StatefulComponent for SearchResultViewer {
    type Props<'a> = SearchResultViewerProps;
    type Output = SearchResultViewerOutput;

    fn render(&mut self, ui: &mut egui::Ui, _props: Self::Props<'_>) -> Self::Output {
        let mut events = Vec::new();

        if self.store.is_empty() {
            // Show empty state
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new(tr!("no-results"))
                        .size(16.0)
                        .color(ui.visuals().weak_text_color())
                );
            });
            return SearchResultViewerOutput { events };
        }

        // Main scroll area for results
        egui::ScrollArea::vertical()
            .id_salt(constants::ID_SALT_SEARCH_RESULT_VIEWER_SCROLL)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                
                let total_items = self.store.len();
                
                for index in 0..total_items {
                    if let Some(hit) = self.store.get_sorted(index).cloned() {
                        let is_selected = self.selected_index == Some(index);
                        
                        let card_result = render_result_card(
                            ui,
                            &hit,
                            index,
                            is_selected,
                            &self.search_mode,
                        );
                        
                        // Check if click was on the file name area using pointer position
                        let clicked_on_filename = if card_result.response.clicked() {
                            if let Some(pos) = ui.ctx().pointer_interact_pos() {
                                card_result.file_name_rect.contains(pos)
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        
                        // Handle file name link click - open the file
                        if clicked_on_filename {
                            tracing::info!("File link clicked! Opening: {:?}", card_result.file_path);
                            match open::that(&card_result.file_path) {
                                Ok(_) => tracing::info!("Successfully opened file"),
                                Err(e) => tracing::error!("Failed to open file {:?}: {}", card_result.file_path, e),
                            }
                        }
                        // Handle card click (not on file name) - select the item
                        else if card_result.response.clicked() {
                            self.selected_index = Some(index);
                            events.push(SearchResultViewerEvent::FileSelected(hit.file_path.clone()));
                        }
                        
                        // Show pointer cursor when hovering over file name
                        if card_result.file_name_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or_default()) {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        
                        // Double-click on card also opens the file
                        if card_result.response.double_clicked() {
                            tracing::info!("Card double-clicked! Opening: {:?}", card_result.file_path);
                            if let Err(e) = open::that(&card_result.file_path) {
                                tracing::error!("Failed to open file {:?}: {}", card_result.file_path, e);
                            }
                        }
                        
                        // Hover effect
                        if card_result.response.hovered() && !is_selected {
                            ui.painter().rect_stroke(
                                card_result.response.rect,
                                egui::CornerRadius::same(6),
                                egui::Stroke::new(1.5, ui.visuals().widgets.hovered.bg_stroke.color),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            });

        // Optional preview panel on the right
        if self.show_preview {
            let max_width = ui.available_width();
            egui::SidePanel::right(constants::ID_PANEL_SEARCH_RESULT_VIEWER)
                .frame(
                    egui::Frame::NONE
                        .inner_margin(egui::vec2(4.0, 2.0))
                        .fill(ui.visuals().panel_fill.gamma_multiply(0.8)),
                )
                .resizable(true)
                .default_width(max_width * 0.38)
                .min_width(max_width * 0.3)
                .max_width(max_width * 0.5)
                .show_animated(ui.ctx(), self.show_preview, |ui| {
                    ui.take_available_space();
                    if let Some(idx) = self.selected_index {
                        if let Some(hit) = self.store.get_sorted(idx) {
                            ui.label(&hit.preview);
                        }
                    } else {
                        ui.label(tr!("select-file-preview"));
                    }
                });
        }

        SearchResultViewerOutput { events }
    }
}
