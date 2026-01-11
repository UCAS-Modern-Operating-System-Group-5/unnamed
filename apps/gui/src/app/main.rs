use super::Scope;
use crate::backend;
use crate::components::{self, prelude::*, StatusBarEvent};
use crate::config::Config;
use crate::constants;
use crate::ui;
use egui::Widget;
use rpc::{Request, search::{SearchMode, SortMode}};
use std::sync::mpsc;
use tracing::{error, info};

pub struct App {
    config: Config,

    s: State,
    ability: Ability,
    search_bar: components::SearchBar,
    status_bar: components::StatusBar,

    tx_request: mpsc::Sender<Request>,
    rx_response: mpsc::Receiver<backend::BackendEvent>,

    c: usize
}

#[derive(Default)]
pub struct State {
    scope: Scope,

    /// Whether this application finishes initialization
    initialized: bool,

    /// Whether in Expand Mode
    expand: bool,
    window_size: egui::Vec2,
    dropped_files: Vec<egui::DroppedFile>,

    search_mode: SearchMode,
    sort_mode: SortMode
}

#[derive(Default)]
pub struct Ability {
    pub recenter: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        let (tx_request, rx_request) = mpsc::channel();
        let (tx_response, rx_response) = mpsc::channel();
        let unix_socket_path = config
            .runtime_dir
            .join(config::constants::UNIX_SOCKET_FILE_NAME);

        backend::spawn_backend(
            rx_request,
            tx_response,
            cc.egui_ctx.clone(),
            unix_socket_path,
        );

        ui::setup_ui(&cc.egui_ctx, &config.ui, config.app.background_alpha);
        Self::setup_i18n();

        #[cfg(debug_assertions)]
        Self::setup_debug_options(&cc.egui_ctx);

        // On Android/Wayland, getting outer position is impossible
        let can_recenter =
            egui::ViewportCommand::center_on_screen(&cc.egui_ctx).is_some();


        let state = State {
            expand: if can_recenter { false } else { true },
            ..Default::default()
        };


        Self {
            config,
            s: state,
            ability: Ability {
                recenter: can_recenter,
            },
            search_bar: Default::default(),
            status_bar: Default::default(),
            tx_request,
            rx_response,

            c: 0
        }
    }

    fn setup_i18n() {
        let en = String::from_utf8_lossy(include_bytes!("../../assets/trans/en.ftl"));
        let zh = String::from_utf8_lossy(include_bytes!("../../assets/trans/zh-hans.ftl"));

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
    pub fn handle_file_drop(&mut self, _ctx: &egui::Context) {
        // pass
    }

    pub fn update_window_title(&self, ctx: &egui::Context) {
        let title = config::constants::APP_NAME.to_string();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub fn render_search_bar(&mut self, ctx: &egui::Context) {
        let props = components::SearchBarProps {
            search_mode: Default::default(),
            draw_separate_line: self.s.expand
        };
        let output = self.search_bar.render(ctx, props);

        for _event in output.events {
            // TODO handle search bar events
        }
    }

    pub fn render_status_bar(&mut self, ctx: &egui::Context) {
        let server_status =
            backend::ServerStatus::Online(backend::ServerWorkingStatus::Searching);
        let props = components::StatusBarProps {
            server_status,
            search_mode: &self.s.search_mode,
            sort_mode: &self.s.sort_mode
        };
        let output = self.status_bar.render(ctx, props);

        for event in output.events {
            match event {
                StatusBarEvent::RestartServer => {
                    
                },
                StatusBarEvent::ChangeSortMode(sort_mode) => {
                    self.s.sort_mode = sort_mode;
                },
            }
        }
    }

    pub fn handle_backend_event(&self) {
        while let Ok(event) = self.rx_response.try_recv() {
            match event {
                backend::BackendEvent::Connected => {
                    info!("Connected to server");
                    let _ = self.tx_request.send(Request::Ping);
                }
                backend::BackendEvent::ConnectionFailed(e) => {
                    error!("Connection to server failed: {e}");
                }
                backend::BackendEvent::RpcResponse(response) => {
                    info!("{response:?}");
                }
            }
        }
    }

    pub fn resize_window(&self, ctx: &egui::Context) {
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
        if !self.s.initialized {
            self.search_bar.request_focus();
            
            self.s.initialized = true;
        }
        
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        }

        // TODO first test it on my Arch Linux XFCE desktop

        self.handle_backend_event();

        self.resize_window(ctx);

        self.update_window_title(ctx);

        self.handle_file_drop(ctx);

        self.render_search_bar(ctx);

        self.render_status_bar(ctx);

        
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE.inner_margin(egui::vec2(4.0, 2.0))
                    .fill(ctx.style().visuals.panel_fill)
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("HI");
                });
            });
    }
}
