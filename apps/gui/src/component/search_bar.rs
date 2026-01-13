use super::ContextComponent;
use crate::constants;
use crate::util::MemoizedQueryHighligher;
use crate::util::completion::{CompletionItem, CompletionSessionId, CompletionState};
use egui::{Sense, text_edit::TextEditOutput};
use rpc::search::SearchMode;

use egui_i18n::tr;
use std::time::{Duration, Instant};

const COMPLETION_VISIBLE_ITEMS_NUM: usize = 10;

#[derive(Default)]
pub struct SearchBar {
    raw_search_query: String,
    panel_height: f32,
    request_focus: bool,
    query_highligher: MemoizedQueryHighligher,

    completion: CompletionState,

    // Debouncing
    last_query: String,
    last_cursor_pos: usize,
    last_change_time: Option<Instant>,
    pending_completion_request: bool,

    current_cursor: usize,
    // The cursor which is set when we don't want completion UI
    ignore_cursor: Option<usize>,

    should_apply_completion: bool,
}

pub struct SearchBarProps<'a> {
    pub search_mode: &'a SearchMode,
    pub draw_separate_line: bool,
}

pub struct SearchBarOutput {
    pub events: Vec<SearchBarEvent>,
}

pub enum SearchBarEvent {
    StartSearch(String),

    RequestCompletion {
        session_id: CompletionSessionId,
        query: String,
        cursor_pos: usize,
    },

    ContinueCompletion {
        session_id: CompletionSessionId,
    },

    CancelCompletion {
        session_id: CompletionSessionId,
    },
}

impl SearchBar {
    pub fn height(&self) -> f32 {
        self.panel_height
    }

    pub fn request_focus(&mut self) {
        self.request_focus = true;
    }

    pub fn query(&self) -> &str {
        &self.raw_search_query
    }

    /// Call this when receiving completion response from backend
    pub fn receive_completion_batch(
        &mut self,
        session_id: CompletionSessionId,
        items: Vec<CompletionItem>,
        has_more: bool,
    ) {
        self.completion.receive_batch(session_id, items, has_more);
    }

    /// Call this when completion is cancelled
    pub fn completion_cancelled(&mut self, session_id: CompletionSessionId) {
        self.completion.cancel(session_id);
    }

    /// Apply selected completion item
    fn apply_completion(
        &mut self,
        ctx: &egui::Context,
        text_edit_output: &TextEditOutput,
    ) {
        fn _apply_completion(
            s: &mut SearchBar,
            ctx: &egui::Context,
            item: CompletionItem,
            text_edit_output: &TextEditOutput,
        ) {
            let new_cursor_pos =
                item.replacement.range.start + item.replacement.text.len();
            s.raw_search_query
                .replace_range(item.replacement.range, &item.replacement.text);
            s.completion.clear();

            let text_edit_id = text_edit_output.response.id;
            if let Some(mut state) = egui::TextEdit::load_state(ctx, text_edit_id) {
                let ccursor = egui::text::CCursor::new(new_cursor_pos);
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                state.store(ctx, text_edit_id);
                s.request_focus = true;
            }
        }

        if let Some(idx) = self.completion.selected {
            if let Some(item) = self.completion.items.get(idx).cloned() {
                _apply_completion(self, ctx, item, text_edit_output);
            }
        }

        if !self.completion.items.is_empty() {
            let item = self.completion.items[0].clone();
            _apply_completion(self, ctx, item, text_edit_output);
        }
    }

    pub fn should_handle_completion(&mut self) -> bool {
        if self.ignore_cursor.is_some_and(|c| c == self.current_cursor) {
            return false;
        }

        if self.completion.items.is_empty() {
            return false;
        }
        return true;
    }

    fn should_request_completion(&mut self, query: &str, cursor_pos: usize) -> bool {
        let query_changed = query != self.last_query;
        let cursor_changed = cursor_pos != self.last_cursor_pos;

        if query_changed || cursor_changed {
            self.last_query = query.to_string();
            self.last_cursor_pos = cursor_pos;
            self.last_change_time = Some(Instant::now());
            self.pending_completion_request = true;
        }

        // Check debounce
        if self.pending_completion_request {
            if let Some(last_change) = self.last_change_time {
                if last_change.elapsed()
                    >= Duration::from_millis(constants::COMPLETION_DEBOUNCE_MS)
                {
                    self.pending_completion_request = false;
                    return true;
                }
            }
        }

        false
    }

    fn handle_completion_keyboard(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<SearchBarEvent> {
        if !self.should_handle_completion() {
            return None;
        }

        let mut event = None;

        ctx.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                self.ignore_cursor = Some(self.current_cursor);
                if let Some(session_id) = self.completion.session_id {
                    event = Some(SearchBarEvent::CancelCompletion { session_id });
                }
                self.completion.clear();
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                self.completion.select_next();
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                self.completion.select_prev();
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::Tab) {
                self.should_apply_completion = true;
            } else if self.completion.selected.is_some()
                && input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
            {
                self.should_apply_completion = true;
            }
        });
        event
    }

    // TODO In non-expanded mode, height is limited
    /// Render completion popup using egui::Popup (like egui_code_editor)
    fn render_completion_popup(
        &mut self,
        ctx: &egui::Context,
        text_edit_output: &TextEditOutput,
    ) -> Option<SearchBarEvent> {
        if !self.should_handle_completion() {
            return None;
        }

        let Some(cursor_range) = text_edit_output.state.cursor.char_range() else {
            return None;
        };
        let galley = &text_edit_output.galley;
        let cursor = cursor_range.primary;
        let cursor_pos_in_galley = galley.pos_from_cursor(cursor);
        let cursor_rect = cursor_pos_in_galley
            .translate(text_edit_output.response.rect.left_top().to_vec2());

        let mut event = None;

        let popup_id = egui::Id::new(constants::ID_SEARCH_BAR_COMPLETION_POPUP);
        let text_edit_rect = text_edit_output.response.rect;
        let layer_id = text_edit_output.response.layer_id;
        let rect = if cursor.index == 0 {
            text_edit_rect
        } else {
            cursor_rect
        };

        egui::Popup::new(popup_id, ctx.clone(), rect, layer_id)
            .frame(egui::Frame::popup(&ctx.style()))
            .sense(Sense::empty())
            .show(|ui| {
                ui.response().sense = Sense::empty();
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                let text_height = ui.text_style_height(&egui::TextStyle::Button);
                let button_padding = ui.style().spacing.button_padding.y * 2.0;
                let item_spacing = ui.style().spacing.item_spacing.y;
                let num_items = self
                    .completion
                    .items
                    .len()
                    .min(COMPLETION_VISIBLE_ITEMS_NUM);
                let single_item_height = text_height + button_padding;
                let height = single_item_height * num_items as f32
                    + item_spacing * num_items.saturating_sub(1) as f32;
                ui.set_height(height);

                egui::ScrollArea::vertical()
                    .auto_shrink([true, true])
                    .scroll_bar_visibility(
                        egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                    )
                    .show(ui, |ui| {
                        for (idx, item) in self.completion.items.iter().enumerate() {
                            let is_selected = self.completion.selected == Some(idx);

                            let button = ui.add(
                                egui::Button::new(&item.label)
                                    .sense(Sense::empty())
                                    .frame(true)
                                    .fill(if is_selected {
                                        ui.style().visuals.widgets.hovered.weak_bg_fill
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    })
                                    .stroke(if is_selected {
                                        ui.style().visuals.widgets.hovered.bg_stroke
                                    } else {
                                        egui::Stroke::NONE
                                    }),
                            );

                            if is_selected {
                                button.scroll_to_me(None);
                            }
                        }

                        // TODO calculation based on the selected index
                        if !self.completion.loading && self.completion.has_more {
                            // Request more when scrolled near bottom
                            if let Some(session_id) = self.completion.session_id {
                                let scroll_offset =
                                    ui.clip_rect().bottom() - ui.min_rect().bottom();
                                if scroll_offset.abs() < 50.0 {
                                    self.completion.loading = true;
                                    event = Some(SearchBarEvent::ContinueCompletion {
                                        session_id,
                                    });
                                }
                            }
                        }
                    });
            });

        event
    }
}

fn setup_text_edit_style(style: &mut egui::Style) {
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.text_cursor.stroke = egui::Stroke::new(4.0, style.visuals.text_color());
}

impl ContextComponent for SearchBar {
    type Props<'a> = SearchBarProps<'a>;
    type Output = SearchBarOutput;

    fn render(&mut self, ctx: &egui::Context, props: Self::Props<'_>) -> Self::Output {
        let mut events = vec![];

        if props.search_mode == &SearchMode::Rule {
            if let Some(event) = self.handle_completion_keyboard(ctx) {
                events.push(event);
            }
        }

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

                ui.scope(|ui| {
                    let style = ui.style_mut();
                    setup_text_edit_style(style);

                    let editor = egui::TextEdit::singleline(&mut self.raw_search_query)
                        .desired_width(f32::INFINITY)
                        .font(
                            egui::TextStyle::Name(
                                constants::TEXT_STYLE_SEARCH_BAR.into(),
                            )
                            .resolve(ui.style()),
                        )
                        .background_color(egui::Color32::TRANSPARENT)
                        .hint_text(hint_text);

                    let output = if props.search_mode == &SearchMode::Rule {
                        let mut layouter =
                            |ui: &egui::Ui,
                             buf: &dyn egui::TextBuffer,
                             wrap_width: f32| {
                                let mut layout_job = self
                                    .query_highligher
                                    .highlight(ui.style(), buf.as_str());
                                layout_job.wrap.max_width = wrap_width;
                                ui.fonts_mut(|f| f.layout_job(layout_job))
                            };
                        editor.layouter(&mut layouter).show(ui)
                    } else {
                        editor.show(ui)
                    };

                    // Handle completion for Rule mode
                    if props.search_mode == &SearchMode::Rule {
                        if let Some(range) = output.cursor_range {
                            if self.should_apply_completion {
                                self.apply_completion(ctx, &output);
                                self.should_apply_completion = false;
                            }

                            let cursor_pos = range.primary.index;

                            if self.current_cursor != cursor_pos {
                                // We don't reset selected since it causes visual jump
                                self.current_cursor = cursor_pos;
                                self.ignore_cursor = None;
                            }

                            let query = self.raw_search_query.clone();

                            if self.should_request_completion(&query, cursor_pos) {
                                if let Some(old_session_id) = self.completion.session_id {
                                    events.push(SearchBarEvent::CancelCompletion {
                                        session_id: old_session_id,
                                    });
                                }

                                let session_id = self.completion.new_session_id();
                                // keep showing old items while loading new
                                self.completion.start_session_preserve_items(session_id);

                                events.push(SearchBarEvent::RequestCompletion {
                                    session_id,
                                    query,
                                    cursor_pos,
                                });
                            }
                        }

                        if let Some(event) = self.render_completion_popup(ctx, &output) {
                            events.push(event);
                        }
                    }

                    // Handle Enter key for search (only if no completion selected)
                    if output.response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && self.completion.selected.is_none()
                    {
                        events.push(SearchBarEvent::StartSearch(
                            self.raw_search_query.clone(),
                        ));
                    }

                    if self.request_focus {
                        output.response.request_focus();
                        self.request_focus = false;
                    }

                    output
                });
            });

        if self.pending_completion_request {
            ctx.request_repaint_after(Duration::from_millis(
                constants::COMPLETION_DEBOUNCE_MS,
            ));
        }

        self.panel_height = resp.response.rect.size().y;

        SearchBarOutput { events }
    }
}
