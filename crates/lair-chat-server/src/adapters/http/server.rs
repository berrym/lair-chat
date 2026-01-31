//! HTTP server setup using Axum.
//!
//! Supports both HTTP and HTTPS (TLS) modes based on configuration.

use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use metrics_exporter_prometheus::PrometheusHandle;

use crate::core::engine::ChatEngine;
use crate::storage::Storage;

use super::routes::create_router_with_metrics;
use super::HttpConfig;

/// HTTP server handle for graceful shutdown.
#[derive(Debug)]
pub struct HttpServer {
    /// Shutdown handle for axum-server (HTTPS mode).
    axum_handle: Option<Handle>,
    /// Abort handle for plain HTTP mode.
    http_abort: Option<tokio::task::AbortHandle>,
}

impl HttpServer {
    /// Start the HTTP server.
    ///
    /// If TLS is configured, starts an HTTPS server using rustls.
    /// Otherwise, starts a plain HTTP server.
    pub async fn start<S: Storage + Clone + 'static>(
        config: HttpConfig,
        engine: Arc<ChatEngine<S>>,
    ) -> std::io::Result<Self> {
        Self::start_with_metrics(config, engine, None).await
    }

    /// Start the HTTP server with optional metrics handle.
    pub async fn start_with_metrics<S: Storage + Clone + 'static>(
        config: HttpConfig,
        engine: Arc<ChatEngine<S>>,
        metrics_handle: Option<PrometheusHandle>,
    ) -> std::io::Result<Self> {
        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

        // Create router with all routes and optional metrics
        let app = create_router_with_metrics(engine, metrics_handle)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .layer(TraceLayer::new_for_http());

        if let Some(tls_config) = config.tls {
            // HTTPS mode with TLS
            let handle = Self::start_https(addr, app, tls_config).await?;
            Ok(Self {
                axum_handle: Some(handle),
                http_abort: None,
            })
        } else {
            // Plain HTTP mode
            let abort_handle = Self::start_http(addr, app).await?;
            Ok(Self {
                axum_handle: None,
                http_abort: Some(abort_handle),
            })
        }
    }

    /// Start plain HTTP server.
    async fn start_http(
        addr: SocketAddr,
        app: axum::Router,
    ) -> std::io::Result<tokio::task::AbortHandle> {
        let listener = TcpListener::bind(addr).await?;
        info!("HTTP server listening on http://{}", addr);
        warn!("TLS is disabled - connections are not encrypted. Enable TLS for production use.");

        let task = tokio::spawn(async move {
            // Use into_make_service_with_connect_info to enable ConnectInfo extraction
            // for WebSocket connections
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("HTTP server failed");
        });

        Ok(task.abort_handle())
    }

    /// Start HTTPS server with TLS.
    async fn start_https(
        addr: SocketAddr,
        app: axum::Router,
        tls_config: super::TlsConfig,
    ) -> std::io::Result<Handle> {
        // Load and parse the certificate and key
        let rustls_config = Self::load_rustls_config(&tls_config)?;

        info!("HTTPS server listening on https://{}", addr);
        info!("  Certificate: {}", tls_config.cert_path.display());
        info!("  Private key: {}", tls_config.key_path.display());

        // Create a handle for graceful shutdown
        let handle = Handle::new();
        let handle_clone = handle.clone();

        tokio::spawn(async move {
            axum_server::bind_rustls(addr, rustls_config)
                .handle(handle_clone)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .expect("HTTPS server failed");
        });

        Ok(handle)
    }

    /// Load rustls configuration from certificate and key files.
    fn load_rustls_config(tls_config: &super::TlsConfig) -> std::io::Result<RustlsConfig> {
        // Read certificate file
        let cert_file = File::open(&tls_config.cert_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Failed to open certificate file '{}': {}",
                    tls_config.cert_path.display(),
                    e
                ),
            )
        })?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Failed to parse certificate file '{}': {}",
                        tls_config.cert_path.display(),
                        e
                    ),
                )
            })?;

        if certs.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "No certificates found in file '{}'",
                    tls_config.cert_path.display()
                ),
            ));
        }

        // Read private key file
        let key_file = File::open(&tls_config.key_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Failed to open private key file '{}': {}",
                    tls_config.key_path.display(),
                    e
                ),
            )
        })?;
        let mut key_reader = BufReader::new(key_file);

        // Try to read as PKCS#8 first, then RSA, then EC
        let key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Failed to parse private key file '{}': {}",
                        tls_config.key_path.display(),
                        e
                    ),
                )
            })?
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "No private key found in file '{}'",
                        tls_config.key_path.display()
                    ),
                )
            })?;

        // Create rustls config synchronously, then wrap in RustlsConfig
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to create TLS configuration: {}", e),
                )
            })?;

        Ok(RustlsConfig::from_config(Arc::new(server_config)))
    }

    /// Gracefully shut down the server.
    pub async fn shutdown(self) {
        if let Some(handle) = self.axum_handle {
            // Graceful shutdown for HTTPS
            handle.graceful_shutdown(Some(std::time::Duration::from_millis(100)));
        }
        if let Some(abort) = self.http_abort {
            // Abort for plain HTTP (less graceful, but works)
            abort.abort();
        }
        // Give the server time to finish pending requests
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
