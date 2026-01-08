mod server_status;

pub use server_status::{ServerStatus, ServerWorkingStatus};

use rpc::{Request, Response, WorldClient};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use tarpc::{client, context, tokio_serde::formats::Bincode};
use tokio::runtime::Runtime;
use tracing::info;

#[derive(Debug)]
pub enum BackendEvent {
    /// The backend successfully connected to the socket
    Connected,
    /// The backend failed to connect (fatal error)
    ConnectionFailed(String),
    /// A response to a specific RPC request
    RpcResponse(Result<Response, String>),
}

pub fn spawn_backend(
    rx_request: mpsc::Receiver<Request>,
    tx_response: mpsc::Sender<BackendEvent>,
    ctx: egui::Context,
    unix_socket_path: PathBuf,
) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        rt.block_on(async move {
            let mut transport = tarpc::serde_transport::unix::connect(
                &unix_socket_path,
                Bincode::default,
            );
            transport.config_mut().max_frame_length(usize::MAX);

            let transport = match transport.await {
                Ok(res) => res,
                Err(e) => {
                    let _ = tx_response.send(BackendEvent::ConnectionFailed(format!(
                        "Could not connect to {unix_socket_path:?}: {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };

            let client = WorldClient::new(client::Config::default(), transport).spawn();

            info!("Backend thread connected to RPC server.");
            let _ = tx_response.send(BackendEvent::Connected);
            ctx.request_repaint();

            while let Ok(req) = rx_request.recv() {
                let tx_response = tx_response.clone();
                let ctx = ctx.clone();
                let client = client.clone();

                tokio::spawn(async move {
                    let response: std::result::Result<Response, String> = match req {
                        Request::Ping => client
                            .ping(context::current())
                            .await
                            .map_err(|e| e.to_string())
                            .map(Response::Ping),
                        Request::StartSearch(_) => todo!(),
                    };

                    let _ = tx_response.send(BackendEvent::RpcResponse(response));
                    ctx.request_repaint();
                });
            }
        });
    });
}
