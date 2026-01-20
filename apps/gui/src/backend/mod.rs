mod server_status;

pub use server_status::{ServerStatus, ServerWorkingStatus};

use rpc::{Request as RpcRequest, Response as RpcResponse, WorldClient};
use std::path::Path;
use tarpc::{client, context, tokio_serde::formats::Bincode};


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
    let mut transport = tarpc::serde_transport::unix::connect(
        unix_socket_path,
        Bincode::default,
    );
    transport.config_mut().max_frame_length(usize::MAX);

    let transport = match transport.await {
        Ok(res) => res,
        Err(e) => {
            return Err(format!(
                "Could not connect to {unix_socket_path:?}: {e}"
            ));
        }
    };

    Ok(WorldClient::new(client::Config::default(), transport).spawn())
}

pub async fn handle_backend_request(
    rpc_client: WorldClient,
    request: RpcRequest
) -> std::result::Result<RpcResponse, client::RpcError> {
    match request {
        RpcRequest::Ping => rpc_client
                .ping(context::current())
                .await
                .map(RpcResponse::Ping),
        RpcRequest::StartSearch(req) => {
            rpc_client.start_search(context::current(), req)
                .await
                .map(RpcResponse::StartSearch)
        },
        RpcRequest::SearchStatus(session_id) => {
            rpc_client.search_status(context::current(), session_id)
                .await
                .map(RpcResponse::SearchStatus)
        },
        RpcRequest::CancelSearch(session_id) => {
            rpc_client.cancel_search(context::current(), session_id)
                .await
                .map(RpcResponse::CancelSearch)
        },
        /// Ui should never send this event
        RpcRequest::FetchSearchResults(_) => unreachable!(),
    }
}
