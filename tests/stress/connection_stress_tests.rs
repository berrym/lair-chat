use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

use lair_chat::client::{
    aes_gcm_encryption::AesGcmEncryption,
    config::ConnectionConfig,
    connection_manager::{ConnectionManager, ConnectionStatus},
    tcp_transport::TcpTransport,
};

// Test configuration constants
const MAX_CONNECTIONS: usize = 1000;
const CONNECTION_BATCH_SIZE: usize = 50;
const MESSAGE_BATCH_SIZE: usize = 100;
const TEST_TIMEOUT: Duration = Duration::from_secs(60);
const STRESS_MESSAGE_SIZE: usize = 1024; // 1KB messages for stress testing

// Helper struct to track test metrics
#[derive(Debug, Default)]
struct StressTestMetrics {
    successful_connections: AtomicUsize,
    failed_connections: AtomicUsize,
    messages_sent: AtomicUsize,
    messages_received: AtomicUsize,
    errors_encountered: AtomicUsize,
}

impl StressTestMetrics {
    fn new() -> Self {
        Self::default()
    }

    fn report(&self) {
        println!("\nStress Test Metrics:");
        println!(
            "Successful Connections: {}",
            self.successful_connections.load(Ordering::Relaxed)
        );
        println!(
            "Failed Connections: {}",
            self.failed_connections.load(Ordering::Relaxed)
        );
        println!(
            "Messages Sent: {}",
            self.messages_sent.load(Ordering::Relaxed)
        );
        println!(
            "Messages Received: {}",
            self.messages_received.load(Ordering::Relaxed)
        );
        println!(
            "Errors Encountered: {}",
            self.errors_encountered.load(Ordering::Relaxed)
        );
    }
}

// Helper function to create a test server that can handle multiple connections
async fn setup_stress_test_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
                while let Ok(n) = socket.try_read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if let Err(_) = socket.try_write(&buf[..n]).await {
                        break;
                    }
                }
            });
        }
    })
}

// Generate random test message
fn generate_test_message(size: usize) -> String {
    use rand::{thread_rng, Rng};
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = thread_rng();
    (0..size)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[tokio::test]
async fn test_massive_concurrent_connections() {
    let metrics = Arc::new(StressTestMetrics::new());
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
    let server = setup_stress_test_server(addr).await;

    let mut connection_handles = Vec::new();
    let total_batches = MAX_CONNECTIONS / CONNECTION_BATCH_SIZE;

    println!("Starting massive concurrent connections test...");
    println!("Total connections to attempt: {}", MAX_CONNECTIONS);
    println!("Batch size: {}", CONNECTION_BATCH_SIZE);

    for batch in 0..total_batches {
        let metrics = Arc::clone(&metrics);
        let batch_handle = tokio::spawn(async move {
            let mut batch_connections = Vec::new();

            for _ in 0..CONNECTION_BATCH_SIZE {
                let config = ConnectionConfig::new(addr);
                let mut manager = ConnectionManager::new(config.clone());
                let transport = TcpTransport::new(config);
                let encryption = AesGcmEncryption::new("test_password");

                manager.set_transport(Box::new(transport));
                manager.set_encryption(Box::new(encryption));

                match manager.connect().await {
                    Ok(_) => {
                        metrics
                            .successful_connections
                            .fetch_add(1, Ordering::Relaxed);
                        batch_connections.push(manager);
                    }
                    Err(_) => {
                        metrics.failed_connections.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }

            // Keep connections alive for a short period
            sleep(Duration::from_secs(1)).await;

            // Cleanup connections
            for mut manager in batch_connections {
                if let Err(_) = manager.disconnect().await {
                    metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        connection_handles.push(batch_handle);
        println!("Launched batch {} of {}", batch + 1, total_batches);

        // Small delay between batches to prevent overwhelming the system
        sleep(Duration::from_millis(100)).await;
    }

    // Wait for all batches to complete
    for handle in connection_handles {
        if let Err(e) = handle.await {
            println!("Batch error: {}", e);
            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
        }
    }

    server.abort();
    metrics.report();

    // Assert acceptable performance
    let success_rate =
        metrics.successful_connections.load(Ordering::Relaxed) as f64 / MAX_CONNECTIONS as f64;
    assert!(
        success_rate >= 0.95,
        "Connection success rate too low: {:.2}%",
        success_rate * 100.0
    );
}

#[tokio::test]
async fn test_connection_stability_under_load() {
    let metrics = Arc::new(StressTestMetrics::new());
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
    let server = setup_stress_test_server(addr).await;

    let test_duration = Duration::from_secs(30);
    let mut handles = Vec::new();

    println!("Starting connection stability test...");
    println!("Test duration: {} seconds", test_duration.as_secs());
    println!("Connections per batch: {}", CONNECTION_BATCH_SIZE);

    let start = std::time::Instant::now();

    while start.elapsed() < test_duration {
        let metrics = Arc::clone(&metrics);
        let handle = tokio::spawn(async move {
            let config = ConnectionConfig::new(addr);
            let mut manager = ConnectionManager::new(config.clone());
            let transport = TcpTransport::new(config);
            let encryption = AesGcmEncryption::new("test_password");

            manager.set_transport(Box::new(transport));
            manager.set_encryption(Box::new(encryption));

            // Connect and send messages
            if let Ok(_) = manager.connect().await {
                metrics
                    .successful_connections
                    .fetch_add(1, Ordering::Relaxed);

                // Send a burst of messages
                let message = generate_test_message(STRESS_MESSAGE_SIZE);
                for _ in 0..MESSAGE_BATCH_SIZE {
                    if manager.send_message(message.clone()).await.is_ok() {
                        metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                    } else {
                        metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
                    }
                }

                let _ = manager.disconnect().await;
            } else {
                metrics.failed_connections.fetch_add(1, Ordering::Relaxed);
            }
        });

        handles.push(handle);
        sleep(Duration::from_millis(100)).await;
    }

    // Wait for all operations to complete
    for handle in handles {
        if let Err(e) = handle.await {
            println!("Handler error: {}", e);
            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
        }
    }

    server.abort();
    metrics.report();

    // Assert acceptable performance
    let message_success_rate = metrics.messages_sent.load(Ordering::Relaxed) as f64
        / (metrics.successful_connections.load(Ordering::Relaxed) * MESSAGE_BATCH_SIZE) as f64;
    assert!(
        message_success_rate >= 0.95,
        "Message success rate too low: {:.2}%",
        message_success_rate * 100.0
    );
}

#[tokio::test]
async fn test_rapid_connect_disconnect() {
    let metrics = Arc::new(StressTestMetrics::new());
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
    let server = setup_stress_test_server(addr).await;

    const CYCLES: usize = 100;
    const CONCURRENT_CLIENTS: usize = 10;

    println!("Starting rapid connect/disconnect test...");
    println!("Cycles: {}", CYCLES);
    println!("Concurrent clients: {}", CONCURRENT_CLIENTS);

    let mut handles = Vec::new();

    for client_id in 0..CONCURRENT_CLIENTS {
        let metrics = Arc::clone(&metrics);
        let handle = tokio::spawn(async move {
            let config = ConnectionConfig::new(addr);
            let mut manager = ConnectionManager::new(config.clone());
            let transport = TcpTransport::new(config);
            let encryption = AesGcmEncryption::new("test_password");

            manager.set_transport(Box::new(transport));
            manager.set_encryption(Box::new(encryption));

            for cycle in 0..CYCLES {
                match manager.connect().await {
                    Ok(_) => {
                        metrics
                            .successful_connections
                            .fetch_add(1, Ordering::Relaxed);
                        if let Err(_) = manager.disconnect().await {
                            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        metrics.failed_connections.fetch_add(1, Ordering::Relaxed);
                    }
                }

                if cycle % 10 == 0 {
                    println!("Client {} completed {} cycles", client_id, cycle);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        if let Err(e) = handle.await {
            println!("Client error: {}", e);
            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
        }
    }

    server.abort();
    metrics.report();

    // Assert acceptable performance
    let total_attempts = CYCLES * CONCURRENT_CLIENTS;
    let success_rate =
        metrics.successful_connections.load(Ordering::Relaxed) as f64 / total_attempts as f64;
    assert!(
        success_rate >= 0.95,
        "Connect/disconnect success rate too low: {:.2}%",
        success_rate * 100.0
    );
}

#[tokio::test]
async fn test_connection_recovery_under_stress() {
    let metrics = Arc::new(StressTestMetrics::new());
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
    let server = setup_stress_test_server(addr).await;

    const RECOVERY_CYCLES: usize = 50;
    const CONCURRENT_CLIENTS: usize = 5;

    println!("Starting connection recovery stress test...");
    println!("Recovery cycles: {}", RECOVERY_CYCLES);
    println!("Concurrent clients: {}", CONCURRENT_CLIENTS);

    let mut handles = Vec::new();

    for client_id in 0..CONCURRENT_CLIENTS {
        let metrics = Arc::clone(&metrics);
        let handle = tokio::spawn(async move {
            let config = ConnectionConfig::new(addr);
            let mut manager = ConnectionManager::new(config.clone());
            let transport = TcpTransport::new(config);
            let encryption = AesGcmEncryption::new("test_password");

            manager.set_transport(Box::new(transport));
            manager.set_encryption(Box::new(encryption));

            for cycle in 0..RECOVERY_CYCLES {
                // Connect and force disconnect to test recovery
                if let Ok(_) = manager.connect().await {
                    metrics
                        .successful_connections
                        .fetch_add(1, Ordering::Relaxed);

                    // Send some messages before forcing disconnect
                    let message = generate_test_message(STRESS_MESSAGE_SIZE);
                    for _ in 0..5 {
                        if manager.send_message(message.clone()).await.is_ok() {
                            metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    // Force disconnect
                    let _ = manager.disconnect().await;

                    // Attempt immediate reconnection
                    if let Ok(_) = manager.connect().await {
                        metrics
                            .successful_connections
                            .fetch_add(1, Ordering::Relaxed);
                    } else {
                        metrics.failed_connections.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    metrics.failed_connections.fetch_add(1, Ordering::Relaxed);
                }

                if cycle % 10 == 0 {
                    println!("Client {} completed {} recovery cycles", client_id, cycle);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        if let Err(e) = handle.await {
            println!("Client error: {}", e);
            metrics.errors_encountered.fetch_add(1, Ordering::Relaxed);
        }
    }

    server.abort();
    metrics.report();

    // Assert acceptable performance
    let total_attempts = RECOVERY_CYCLES * CONCURRENT_CLIENTS * 2; // *2 for initial connect and recovery
    let success_rate =
        metrics.successful_connections.load(Ordering::Relaxed) as f64 / total_attempts as f64;
    assert!(
        success_rate >= 0.90,
        "Recovery success rate too low: {:.2}%",
        success_rate * 100.0
    );
}
