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
                spawn_search_poller(
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

fn spawn_search_poller(
    rpc_client: WorldClient,
    session_id: Uuid,
    tx_response: mpsc::Sender<Response>,
    egui_ctx: egui::Context,
) {
    tokio::spawn(async move {
        poll_search_status_and_results(rpc_client, session_id, tx_response, egui_ctx)
            .await;
    });
}

/// Periodically polls search status and fetches results until the search
/// completes, fails, or is cancelled.
async fn poll_search_status_and_results(
    rpc_client: WorldClient,
    session_id: Uuid,
    tx_response: mpsc::Sender<Response>,
    egui_ctx: egui::Context,
) {
    const POLL_INTERVAL: Duration =
        Duration::from_millis(constants::SEARCH_POLL_INTERVAL_MS);
    const BATCH_LIMIT: usize = constants::SEARCH_RESULT_FETCH_BATCH_SIZE;

    let mut offset = 0usize;
    let mut all_results_fetched = false;

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        let status = match fetch_and_send_status(
            &rpc_client,
            session_id,
            &tx_response,
            &egui_ctx,
        )
        .await
        {
            Ok(status) => status,
            Err(PollError::ChannelClosed) => break,
            Err(PollError::RpcError(e)) => {
                let _ = tx_response.send(Response::Backend(BackendEvent::RpcFailure(e)));
                egui_ctx.request_repaint();
                break;
            }
        };

        let search_in_progress = matches!(status, (_, Ok(SearchStatus::InProgress { .. })));
        let search_completed = matches!(status, (_, Ok(SearchStatus::Completed { .. })));

        // Stop immediately for failed/cancelled/error
        if !search_in_progress && !search_completed {
            break;
        }

        if !all_results_fetched {
            match fetch_and_send_results(
                &rpc_client,
                session_id,
                offset,
                BATCH_LIMIT,
                &tx_response,
                &egui_ctx,
            )
            .await
            {
                Ok(FetchOutcome::Fetched { count, has_more }) => {
                    offset += count;
                    if !has_more {
                        all_results_fetched = true;
                    }
                }
                Ok(FetchOutcome::SearchError) => {
                    // Search-level error (session not found, etc.)
                    break;
                }
                Err(PollError::ChannelClosed) => break,
                Err(PollError::RpcError(e)) => {
                    let _ =
                        tx_response.send(Response::Backend(BackendEvent::RpcFailure(e)));
                    egui_ctx.request_repaint();
                    break;
                }
            }
        }

        if search_completed && all_results_fetched {
            break;
        }
    }
}

enum PollError {
    ChannelClosed,
    RpcError(client::RpcError),
}

enum FetchOutcome {
    Fetched { count: usize, has_more: bool },
    SearchError,
}

/// Fetches the search status and sends it to the UI.
async fn fetch_and_send_status(
    rpc_client: &WorldClient,
    session_id: Uuid,
    tx_response: &mpsc::Sender<Response>,
    egui_ctx: &egui::Context,
) -> Result<(Uuid, rpc::search::SResult<SearchStatus>), PollError> {
    let result = rpc_client
        .search_status(context::current(), session_id)
        .await
        .map_err(PollError::RpcError)?;

    let response = Response::Backend(BackendEvent::RpcResponse(
        RpcResponse::SearchStatus(result.clone()),
    ));

    if tx_response.send(response).is_err() {
        return Err(PollError::ChannelClosed);
    }
    egui_ctx.request_repaint();

    Ok(result)
}

/// Fetches search results and sends them to the UI.
async fn fetch_and_send_results(
    rpc_client: &WorldClient,
    session_id: Uuid,
    offset: usize,
    limit: usize,
    tx_response: &mpsc::Sender<Response>,
    egui_ctx: &egui::Context,
) -> Result<FetchOutcome, PollError> {
    let req = FetchSearchResultsRequest {
        session_id,
        offset,
        limit,
    };

    let result = rpc_client
        .fetch_search_results(context::current(), req)
        .await
        .map_err(PollError::RpcError)?;

    let response = Response::Backend(BackendEvent::RpcResponse(
        RpcResponse::FetchSearchResults(result.clone()),
    ));

    if tx_response.send(response).is_err() {
        return Err(PollError::ChannelClosed);
    }
    egui_ctx.request_repaint();

    match result {
        (_, Ok(fetch)) => Ok(FetchOutcome::Fetched {
            count: fetch.hits.len(),
            has_more: fetch.has_more,
        }),
        (_, Err(_)) => Ok(FetchOutcome::SearchError),
    }
}
