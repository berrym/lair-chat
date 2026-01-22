//! TCP server listener.
//!
//! Accepts TCP connections and spawns handlers for each.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{error, info};

use crate::core::engine::ChatEngine;
use crate::storage::Storage;

use super::connection::Connection;
use super::TcpConfig;

/// TCP server handle for graceful shutdown.
pub struct TcpServer {
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
}

impl TcpServer {
    /// Start the TCP server.
    pub async fn start<S: Storage + 'static>(
        config: TcpConfig,
        engine: Arc<ChatEngine<S>>,
    ) -> std::io::Result<Self> {
        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
        let listener = TcpListener::bind(addr).await?;

        info!("TCP server listening on {}", addr);

        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let shutdown_rx = shutdown_tx.subscribe();

        // Spawn the accept loop
        tokio::spawn(Self::accept_loop(listener, engine, shutdown_rx));

        Ok(Self { shutdown_tx })
    }

    /// Accept loop - runs until shutdown.
    async fn accept_loop<S: Storage + 'static>(
        listener: TcpListener,
        engine: Arc<ChatEngine<S>>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let engine = engine.clone();
                            tokio::spawn(async move {
                                Connection::handle(stream, addr, engine).await;
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                // Shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("TCP server shutting down");
                    break;
                }
            }
        }
    }

    /// Gracefully shut down the server.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        // Give connections time to close
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
