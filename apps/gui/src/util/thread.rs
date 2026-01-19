use crate::app::{Request, Response};
use crate::backend::{BackendEvent, handle_backend_request, init_trpc_client};
use crate::util::completion::{CompletionManager, CompletionRequest, CompletionResponse};
use rpc::WorldClient;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

pub struct UniversalEventHandlerThread {
    rpc_unix_socket_path: PathBuf,
    rx_request: mpsc::Receiver<Request>,
    tx_response: mpsc::Sender<Response>,
    egui_ctx: egui::Context,
}

impl UniversalEventHandlerThread {
    pub fn new(
        rpc_unix_socket_path: impl Into<PathBuf>,
        rx_request: mpsc::Receiver<Request>,
        tx_response: mpsc::Sender<Response>,
        egui_ctx: egui::Context,
    ) -> Self {
        Self {
            rpc_unix_socket_path: rpc_unix_socket_path.into(),
            rx_request,
            tx_response,
            egui_ctx,
        }
    }

    pub fn spawn(self) {
        let Self {
            rpc_unix_socket_path,
            rx_request,
            tx_response,
            egui_ctx,
        } = self;

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(e) => {
                    error!("Failed to create Tokio runtime: {}", e);
                    let _ = tx_response.send(Response::SpawnUniversalEventHandlerThreadFailed);
                    return;
                }
            };

            rt.block_on(async move {
                // Initialize completion manager
                let completion_manager = Arc::new(
                    CompletionManager::with_current_dir()
                        .unwrap_or_else(|_| CompletionManager::new("/"))
                );

                let rpc_client = match init_trpc_client(&rpc_unix_socket_path).await {
                    Ok(client) => Some(client),
                    Err(e) => {
                        error!(e);
                        let _ = tx_response.send(Response::Backend(BackendEvent::ConnectionFailed(e)));
                        egui_ctx.request_repaint();
                        None
                    }
                };

                if rpc_client.is_some() {
                    info!("Backend thread connected to RPC server.");
                    let _ = tx_response.send(Response::Backend(BackendEvent::Connected));
                    egui_ctx.request_repaint();
                }

                while let Ok(req) = rx_request.recv() {
                    let tx_response = tx_response.clone();
                    let egui_ctx = egui_ctx.clone();
                    let rpc_client = rpc_client.clone();
                    let completion_manager = completion_manager.clone();

                    tokio::spawn(async move {
                        handle_request(
                            rpc_client,
                            completion_manager,
                            tx_response,
                            req,
                            egui_ctx,
                        ).await;
                    });
                }
            });
        });
    }
}

pub async fn handle_request(
    rpc_client: Option<WorldClient>,
    completion_manager: Arc<CompletionManager>,
    tx_response: mpsc::Sender<Response>,
    request: Request,
    egui_ctx: egui::Context,
) {
    let res: Result<Response, String> = match request {
        Request::Backend(request) => {
            if let Some(rpc_client) = rpc_client {
                let rpc_response = handle_backend_request(rpc_client, request).await;
                match rpc_response {
                    Ok(resp) => Ok(Response::Backend(BackendEvent::RpcResponse(resp))),
                    Err(e) => Ok(Response::Backend(BackendEvent::RpcFailure(e))),
                }
            } else {
                Err("RPC client is not connected to unix socket".into())
            }
        }
        Request::Completion(completion_req) => {
            let response = handle_completion_request(&completion_manager, completion_req).await;
            Ok(Response::Completion(response))
        }
    };

    match res {
        Ok(response) => {
            let _ = tx_response.send(response);
            egui_ctx.request_repaint();
        }
        Err(e) => {
            let _ = tx_response.send(Response::Failure(e));
            egui_ctx.request_repaint();
        }
    }
}

async fn handle_completion_request(
    manager: &CompletionManager,
    request: CompletionRequest,
) -> CompletionResponse {
    match request {
        CompletionRequest::StartCompletion {
            session_id,
            query,
            cursor_pos,
        } => {
            manager.start_session(session_id, &query, cursor_pos).await
        }
        CompletionRequest::ContinueCompletion { session_id } => {
            manager.continue_session(session_id).await
        }
        CompletionRequest::CancelCompletion { session_id } => {
            manager.cancel_session(session_id).await;
            CompletionResponse::Cancelled { session_id }
        }
    }
}
