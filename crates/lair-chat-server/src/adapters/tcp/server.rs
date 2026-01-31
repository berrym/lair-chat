//! TCP server listener.
//!
//! Accepts TCP connections and spawns handlers for each.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::core::engine::ChatEngine;
use crate::storage::Storage;

use super::connection::Connection;
use super::TcpConfig;

/// TCP server handle for graceful shutdown.
pub struct TcpServer {
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
    /// Current connection count (shared with accept loop).
    connection_count: Arc<AtomicU32>,
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
        if config.max_connections > 0 {
            info!("  Max connections: {}", config.max_connections);
        } else {
            info!("  Max connections: unlimited");
        }

        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let shutdown_rx = shutdown_tx.subscribe();
        let connection_count = Arc::new(AtomicU32::new(0));

        // Spawn the accept loop
        tokio::spawn(Self::accept_loop(
            listener,
            engine,
            shutdown_rx,
            connection_count.clone(),
            config.max_connections,
        ));

        Ok(Self {
            shutdown_tx,
            connection_count,
        })
    }

    /// Accept loop - runs until shutdown.
    async fn accept_loop<S: Storage + 'static>(
        listener: TcpListener,
        engine: Arc<ChatEngine<S>>,
        mut shutdown_rx: broadcast::Receiver<()>,
        connection_count: Arc<AtomicU32>,
        max_connections: u32,
    ) {
        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            // Check connection limit (0 means unlimited)
                            if max_connections > 0 {
                                let current = connection_count.load(Ordering::Relaxed);
                                if current >= max_connections {
                                    warn!(
                                        "Connection limit reached ({}/{}), rejecting {}",
                                        current, max_connections, addr
                                    );
                                    // Drop stream to close connection immediately
                                    drop(stream);
                                    continue;
                                }
                            }

                            // Increment connection count
                            connection_count.fetch_add(1, Ordering::Relaxed);
                            let count = connection_count.clone();

                            let engine = engine.clone();
                            tokio::spawn(async move {
                                Connection::handle(stream, addr, engine).await;
                                // Decrement connection count when handler exits
                                count.fetch_sub(1, Ordering::Relaxed);
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

    /// Get the current number of active connections.
    pub fn connection_count(&self) -> u32 {
        self.connection_count.load(Ordering::Relaxed)
    }

    /// Gracefully shut down the server.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        // Give connections time to close
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
