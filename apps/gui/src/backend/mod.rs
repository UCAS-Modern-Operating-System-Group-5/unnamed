mod server_status;

pub use server_status::{ServerStatus, ServerWorkingStatus};

use crate::app::Response;
use crate::constants;
use rpc::search::{FetchSearchResultsRequest, SearchStatus};
use rpc::{Request as RpcRequest, Response as RpcResponse, WorldClient};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tarpc::{client, context, tokio_serde::formats::Bincode};
use uuid::Uuid;

#[derive(Debug)]
pub enum BackendEvent {
    /// The backend successfully connected to the socket
    Connected,
    /// The backend failed to connect (fatal error)
    ConnectionFailed(String),
    /// A response to a specific RPC request
    RpcResponse(RpcResponse),
    RpcFailure(client::RpcError),
}

pub async fn init_trpc_client(
    unix_socket_path: &Path,
) -> std::result::Result<WorldClient, String> {
    let mut transport =
        tarpc::serde_transport::unix::connect(unix_socket_path, Bincode::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let transport = match transport.await {
        Ok(res) => res,
        Err(e) => {
            return Err(format!("Could not connect to {unix_socket_path:?}: {e}"));
        }
    };

    Ok(WorldClient::new(client::Config::default(), transport).spawn())
}

pub async fn handle_backend_request(
    rpc_client: WorldClient,
    request: RpcRequest,
    tx_response: mpsc::Sender<Response>,
    egui_ctx: egui::Context,
) -> std::result::Result<RpcResponse, client::RpcError> {
    match request {
        RpcRequest::Ping => rpc_client
            .ping(context::current())
            .await
            .map(RpcResponse::Ping),

        RpcRequest::StartSearch(req) => {
            let result = rpc_client.start_search(context::current(), req).await;

            if let Ok(Ok(session_id)) = &result {
                spawn_auto_fetcher(
                    rpc_client.clone(),
                    *session_id,
                    tx_response,
                    egui_ctx,
                );
            }

            result.map(RpcResponse::StartSearch)
        }

        RpcRequest::SearchStatus(session_id) => rpc_client
            .search_status(context::current(), session_id)
            .await
            .map(RpcResponse::SearchStatus),

        RpcRequest::CancelSearch(session_id) => rpc_client
            .cancel_search(context::current(), session_id)
            .await
            .map(RpcResponse::CancelSearch),

        // UI should never send this event directly
        RpcRequest::FetchSearchResults(_) => unreachable!(),
    }
}

fn spawn_auto_fetcher(
    rpc_client: WorldClient,
    session_id: Uuid,
    tx_response: mpsc::Sender<Response>,
    egui_ctx: egui::Context,
) {
    tokio::spawn(async move {
        auto_fetch_search_results(rpc_client, session_id, tx_response, egui_ctx).await;
    });
}

/// Automatically fetches search results periodically until the search completes, fails,
/// or is cancelled.
async fn auto_fetch_search_results(
    rpc_client: WorldClient,
    session_id: Uuid,
    tx_response: mpsc::Sender<Response>,
    egui_ctx: egui::Context,
) {
    const FETCH_INTERVAL: Duration =
        Duration::from_millis(constants::SEARCH_RESULT_FETCH_INTERVAL_MS);
    const BATCH_LIMIT: usize = constants::SEARCH_RESULT_FETCH_BATCH_SIZE;

    let mut offset = 0usize;

    loop {
        tokio::time::sleep(FETCH_INTERVAL).await;

        let req = FetchSearchResultsRequest {
            session_id,
            offset,
            limit: BATCH_LIMIT,
        };

        match rpc_client
            .fetch_search_results(context::current(), req)
            .await
        {
            Ok(result) => {
                let response = Response::Backend(BackendEvent::RpcResponse(
                    RpcResponse::FetchSearchResults(result.clone()),
                ));

                if tx_response.send(response).is_err() {
                    // Receiver dropped, stop fetching
                    break;
                }
                egui_ctx.request_repaint();

                match result {
                    Ok(fetch) => {
                        offset += fetch.hits.len();

                        if !fetch.has_more {
                            break;
                        }
                    }
                    Err(_search_error) => {
                        break;
                    }
                }
            }
            Err(rpc_error) => {
                let _ = tx_response
                    .send(Response::Backend(BackendEvent::RpcFailure(rpc_error)));
                egui_ctx.request_repaint();
                break;
            }
        }
    }
}
