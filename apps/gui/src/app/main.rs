use super::Scope;
use super::{KeyHandler, UserCommand};
use crate::backend::{BackendEvent, ServerStatus, ServerWorkingStatus};
use crate::component::{
    SearchBar, SearchBarEvent, SearchBarProps, SearchResultViewer,
    SearchResultViewerEvent, SearchResultViewerProps, StatusBar, StatusBarEvent,
    StatusBarProps, prelude::*,
};
use crate::config::Config;
use crate::constants;
use crate::ui;
use crate::util::{
    SearchStatus, SortConfig, UniversalEventHandlerThread, WorkingSearchStatus,
    completion::{CompletionRequest, CompletionResponse},
};
use rpc::{
    Request as RpcRequest,
    search::{SResult, SearchMode, SearchRequest},
};
use std::sync::mpsc;
use strum::IntoEnumIterator;
use tarpc::client::RpcError;
use tracing::{error, info};
use uuid::Uuid;

pub struct App {
    // config: Config,
    s: State,

    key_handler: KeyHandler,

    ability: Ability,
    search_bar: SearchBar,
    status_bar: StatusBar,
    search_result_viewer: SearchResultViewer,

    tx_request: mpsc::Sender<Request>,
    rx_response: mpsc::Receiver<Response>,

    c: usize,
}

#[derive(Default)]
pub struct State {
    request_search_focus: bool,

    search_status: SearchStatus,

    server_online: bool,

    /// Whether in Expand Mode
    expand: bool,
    // dropped_files: Vec<egui::DroppedFile>,
    search_mode: SearchMode,
    // sort_mode: SortMode,
    sort_config: SortConfig,
}

#[derive(Default)]
pub struct Ability {
    pub recenter: bool,
}

pub enum Request {
    Backend(RpcRequest),
    Completion(CompletionRequest),
}

pub enum Response {
    SpawnUniversalEventHandlerThreadFailed,
    /// Generic failure with reason
    Failure(String),
    Backend(BackendEvent),
    Completion(CompletionResponse),
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        let (tx_request, rx_request) = mpsc::channel();
        let (tx_response, rx_response) = mpsc::channel();
        let unix_socket_path = config
            .runtime_dir
            .join(config::constants::UNIX_SOCKET_FILE_NAME);

        UniversalEventHandlerThread::new(
            unix_socket_path,
            rx_request,
            tx_response,
            cc.egui_ctx.clone(),
        )
        .spawn();

        ui::setup_ui(&cc.egui_ctx, &config.ui, config.app.background_alpha);
        Self::setup_i18n();

        #[cfg(debug_assertions)]
        Self::setup_debug_options(&cc.egui_ctx);

        // On Android/Wayland, getting outer position is impossible
        let can_recenter =
            egui::ViewportCommand::center_on_screen(&cc.egui_ctx).is_some();

        let state = State {
            expand: if can_recenter { false } else { true },
            request_search_focus: true,
            // search_mode: SearchMode::Rule,
            ..Default::default()
        };

        let search_bar = Self::create_search_bar(&config);

        Self {
            s: state,
            key_handler: KeyHandler::new(config.keys),
            ability: Ability {
                recenter: can_recenter,
            },
            search_bar,
            status_bar: Default::default(),
            search_result_viewer: Default::default(),
            tx_request,
            rx_response,

            c: 0,
        }
    }

    fn create_search_bar(config: &Config) -> SearchBar {
        // Find start_search key shortcut
        let mut start_search_key_shortcut = None;
        let mut completion_start_search_key_shortcut = None;
        for (scope, bindings) in &config.keys {
            match scope {
                Scope::SearchBar => {
                    for (key_shortcut, user_command) in bindings {
                        if user_command == &UserCommand::StartSearch {
                            start_search_key_shortcut = Some(key_shortcut.clone());
                        }
                    }
                }
                Scope::SearchBarCompletion => {
                    for (key_shortcut, user_command) in bindings {
                        if user_command == &UserCommand::StartSearch {
                            completion_start_search_key_shortcut =
                                Some(key_shortcut.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        SearchBar::new(
            egui::Id::new(constants::ID_SEARCH_BAR_INPUT),
            start_search_key_shortcut,
            completion_start_search_key_shortcut,
        )
    }

    /// Tweaking Egui's beheavior to make it suitable for this application.
    /// For example, Egui will always move focus when user pressing TAB even if
    /// We have consumed the TAB key. We need to tweak this beheavior.
    fn tweak_egui_beheavior(&self, ctx: &egui::Context) {
        // Make TAB key don't move focus
        // Reference: https://github.com/emilk/egui/issues/5878#issuecomment-3316326257
        if let Some(focused_widget) = ctx.memory(|i| i.focused()) {
            ctx.memory_mut(|mem| {
                mem.set_focus_lock_filter(
                    focused_widget,
                    egui::EventFilter {
                        tab: true,
                        ..Default::default()
                    },
                );
            });
        }
    }

    fn current_scope(&self, ctx: &egui::Context) -> Scope {
        ctx.memory(|mem| match mem.focused() {
            Some(id) if id == self.search_bar.id() => self.search_bar.current_scope(),
            Some(_) => Scope::Main,
            None => Scope::Global,
        })
    }

    fn handle_key(&mut self, ctx: &egui::Context) {
        let cur_scope = self.current_scope(ctx);
        for (scope, cmd) in self.key_handler.handle(ctx, &cur_scope) {
            let handled = match scope {
                Scope::Global => false,
                Scope::Main => {
                    if self.s.expand {
                        self.search_result_viewer.handle_user_command(&cmd)
                    } else {
                        false
                    }
                }
                Scope::SearchBar | Scope::SearchBarCompletion => {
                    self.search_bar.handle_user_command(&scope, &cmd)
                }
            };

            if !handled {
                self.handle_user_command(ctx, cmd);
            }
        }
    }

    /// Handle user command at Global scope.
    fn handle_user_command(&mut self, ctx: &egui::Context, cmd: UserCommand) {
        match cmd {
            UserCommand::QuitApplication => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            UserCommand::ToggleFullScreen => {
                let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
            }
            UserCommand::ToggleSearchMode => {
                let mode = SearchMode::iter()
                    .cycle()
                    .skip_while(|m| m != &self.s.search_mode)
                    .skip(1)
                    .next()
                    .unwrap();
                self.change_search_mode(mode);
            }
            _ => {}
        }
    }

    fn setup_i18n() {
        let en = String::from_utf8_lossy(include_bytes!("../../assets/trans/en.ftl"));
        let zh =
            String::from_utf8_lossy(include_bytes!("../../assets/trans/zh-hans.ftl"));

        // Note, it should panic if we cannot display text on the UI
        egui_i18n::load_translations_from_text("en", en).unwrap();
        egui_i18n::load_translations_from_text("zh", zh).unwrap();

        // TODO Should detect system locale
        egui_i18n::set_language("zh");
        egui_i18n::set_fallback("en");
    }

    #[cfg(debug_assertions)]
    fn setup_debug_options(ctx: &egui::Context) {
        ctx.style_mut(|style| style.debug.debug_on_hover_with_all_modifiers = true);
    }

    /// Should set search path to the parent directory of the file; Or if the dropped
    /// stuff is a directory, then set the search path to that directory
    fn handle_file_drop(&mut self, _ctx: &egui::Context) {
        // pass
    }

    fn update_window_title(&self, ctx: &egui::Context) {
        let title = config::constants::APP_NAME.to_string();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn render_search_bar(&mut self, ctx: &egui::Context) {
        let props = SearchBarProps {
            search_mode: &self.s.search_mode,
            draw_separate_line: self.s.expand,
        };
        let output = self.search_bar.render(ctx, props);
        for event in output.events {
            match event {
                SearchBarEvent::StartSearch(query) => {
                    if !self.s.server_online {
                        continue;
                    }
                    let search_request = SearchRequest {
                        query,
                        search_mode: self.s.search_mode.clone(),
                    };
                    let _ = self
                        .tx_request
                        .send(Request::Backend(RpcRequest::StartSearch(search_request)));
                }
                SearchBarEvent::RequestCompletion {
                    session_id,
                    query,
                    cursor_pos,
                } => {
                    let _ = self.tx_request.send(Request::Completion(
                        CompletionRequest::StartCompletion {
                            session_id,
                            query,
                            cursor_pos,
                        },
                    ));
                }
                SearchBarEvent::ContinueCompletion { session_id } => {
                    let _ = self.tx_request.send(Request::Completion(
                        CompletionRequest::ContinueCompletion { session_id },
                    ));
                }
                SearchBarEvent::CancelCompletion { session_id } => {
                    let _ = self.tx_request.send(Request::Completion(
                        CompletionRequest::CancelCompletion { session_id },
                    ));
                }
            }
        }
    }

    fn render_search_result_viewer(&mut self, ui: &mut egui::Ui) {
        let props = SearchResultViewerProps {};
        let output = self.search_result_viewer.render(ui, props);
        for event in output.events {}
    }

    fn change_sort_config(&mut self, config: SortConfig) {
        self.search_result_viewer.set_sort_config(config.clone());
        self.s.sort_config = config;
    }

    fn change_search_mode(&mut self, mode: SearchMode) {
        self.search_result_viewer.set_search_mode(mode.clone());
        self.s.search_mode = mode;
        self.s.request_search_focus = true;
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        let server_status = ServerStatus::Online(ServerWorkingStatus::Searching);
        let props = StatusBarProps {
            server_status,
            search_mode: &self.s.search_mode,
            sort_config: &self.s.sort_config,
        };
        let output = self.status_bar.render(ctx, props);

        for event in output.events {
            match event {
                StatusBarEvent::RestartServer => {}
                StatusBarEvent::ChangeSortConfig(config) => {
                    self.change_sort_config(config);
                }
                StatusBarEvent::ChangeSearchMode(mode) => {
                    self.change_search_mode(mode);
                }
            }
        }
    }

    fn handle_event(&mut self) {
        while let Ok(event) = self.rx_response.try_recv() {
            match event {
                Response::SpawnUniversalEventHandlerThreadFailed => {
                    // TODO: handle
                }
                Response::Failure(e) => {
                    error!(e);
                }
                Response::Backend(event) => {
                    self.handle_backend_event(event);
                }
                Response::Completion(response) => {
                    self.handle_completion_response(response);
                }
            }
        }
    }

    fn handle_completion_response(&mut self, response: CompletionResponse) {
        match response {
            CompletionResponse::Batch {
                session_id,
                items,
                has_more,
                total_so_far: _,
            } => {
                self.search_bar
                    .receive_completion_batch(session_id, items, has_more);
            }
            CompletionResponse::Cancelled { session_id } => {
                self.search_bar.completion_cancelled(session_id);
            }
        }
    }

    fn handle_backend_event(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::Connected => {
                info!("Connected to server");
                let _ = self.tx_request.send(Request::Backend(RpcRequest::Ping));
            }
            BackendEvent::RpcFailure(rpc_error) => {
                match rpc_error {
                    RpcError::Shutdown => {
                        self.s.server_online = false;
                        // TODO try reconnection
                    }
                    _ => {
                        // TODO handle other type of errors
                    }
                }
                // TODO Display failture reason
            }
            BackendEvent::ConnectionFailed(e) => {
                // error!("Connection to server failed: {e}");
            }
            BackendEvent::RpcResponse(response) => match response {
                rpc::Response::Ping(_) => {
                    self.s.server_online = true;
                }
                rpc::Response::StartSearch(session_id) => {
                    if let Some(session_id) = self.unwrap_sresult_safe(session_id) {
                        self.s.search_status =
                            SearchStatus::Working(WorkingSearchStatus::new(session_id));
                    }
                }
                rpc::Response::SearchStatus(search_status) => {
                    if let Some(status) = self.unwrap_sresult_safe(search_status) {
                        match status {
                            rpc::search::SearchStatus::Failed(err) => {
                                self.s.search_status = SearchStatus::Failed(err);
                            },
                            rpc::search::SearchStatus::Cancelled => {
                                self.s.search_status = SearchStatus::Idle;
                            },
                            _ => {
                                match &self.s.search_status {
                                    SearchStatus::Working(ss) => {
                                        self.s.search_status =
                                    SearchStatus::Working(WorkingSearchStatus {
                                        session_id: ss.session_id,
                                        status: Some(status),
                                    })
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                rpc::Response::FetchSearchResults(fetch_result) => {
                    if let Some(res) = self.unwrap_sresult_safe(fetch_result) {
                        self.search_result_viewer.recieve_items(res.hits);
                    }
                },
                rpc::Response::CancelSearch(_) => {
                    self.s.search_status = SearchStatus::Idle;
                },
            },
        }
    }

    /// Unwrap the search result and handles error cases
    fn unwrap_sresult_safe<T: std::fmt::Debug>(
        &mut self,
        result: SResult<T>,
    ) -> Option<T> {
        if result.is_ok() {
            return result.ok();
        }

        let err = result.unwrap_err();

        self.s.search_status = SearchStatus::Failed(err);

        None
    }

    fn resize_window(&self, ctx: &egui::Context) {
        if !self.ability.recenter {
            return;
        }

        let height = self.status_bar.height() + self.search_bar.height()
            - ctx.style().visuals.widgets.noninteractive.bg_stroke.width;

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            800.0, height,
        )));

        if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
            ctx.send_viewport_cmd(cmd);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // let mut color = egui::Color32::from(visuals.panel_fill);
        // color = color.gamma_multiply(self.config.app.background_alpha);
        // color.to_normalized_gamma_f32()
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.s.request_search_focus {
            self.search_bar.request_focus();

            self.s.request_search_focus = false;
        }

        self.tweak_egui_beheavior(ctx);
        self.handle_key(ctx);

        // TODO first test it on my Arch Linux XFCE desktop

        self.handle_event();
        self.resize_window(ctx);
        self.update_window_title(ctx);
        self.handle_file_drop(ctx);
        self.render_search_bar(ctx);
        self.render_status_bar(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .inner_margin(egui::vec2(4.0, 2.0))
                    .fill(ctx.style().visuals.panel_fill),
            )
            .show(ctx, |ui| {
                self.render_search_result_viewer(ui);
            });
    }
}
