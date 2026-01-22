//! HTTP server setup using Axum.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::core::engine::ChatEngine;
use crate::storage::Storage;

use super::routes::create_router;
use super::HttpConfig;

/// HTTP server handle for graceful shutdown.
pub struct HttpServer {
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
}

impl HttpServer {
    /// Start the HTTP server.
    pub async fn start<S: Storage + Clone + 'static>(
        config: HttpConfig,
        engine: Arc<ChatEngine<S>>,
    ) -> std::io::Result<Self> {
        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

        // Create router with all routes
        let app = create_router(engine)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .layer(TraceLayer::new_for_http());

        let listener = TcpListener::bind(addr).await?;
        info!("HTTP server listening on {}", addr);

        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        // Spawn the server
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.recv().await;
                })
                .await
                .expect("HTTP server failed");
        });

        Ok(Self { shutdown_tx })
    }

    /// Gracefully shut down the server.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        // Give the server time to finish pending requests
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
