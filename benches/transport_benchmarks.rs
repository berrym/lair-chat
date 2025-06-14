use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Runtime;

use lair_chat::client::{
    aes_gcm_encryption::AesGcmEncryption, config::ConnectionConfig,
    connection_manager::ConnectionManager, tcp_transport::TcpTransport,
};

async fn setup_test_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                while let Ok(n) = socket.try_read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if socket.try_write(&buf[..n]).is_err() {
                        break;
                    }
                }
            });
        }
    })
}

fn connection_establishment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("connection_establishment");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    group.bench_function("establish_connection", |b| {
        b.to_async(&rt).iter(|| async {
            let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
            let server = setup_test_server(addr).await;

            let config = ConnectionConfig::new(addr);
            let mut manager = ConnectionManager::new(config.clone());
            let transport = TcpTransport::new(config);
            let encryption = AesGcmEncryption::new("test_password");

            manager.set_transport(Box::new(transport));
            manager.set_encryption(Box::new(encryption));

            let _ = manager.connect().await;
            let _ = manager.disconnect().await;

            server.abort();
        });
    });

    group.finish();
}

fn message_roundtrip(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let message_sizes = vec![64, 256, 1024, 4096];
    let mut group = c.benchmark_group("message_roundtrip");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    for size in message_sizes {
        group.bench_with_input(BenchmarkId::new("message_size", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
                let server = setup_test_server(addr).await;

                let config = ConnectionConfig::new(addr);
                let mut manager = ConnectionManager::new(config.clone());
                let transport = TcpTransport::new(config);
                let encryption = AesGcmEncryption::new("test_password");

                manager.set_transport(Box::new(transport));
                manager.set_encryption(Box::new(encryption));

                let _ = manager.connect().await;

                let message = "A".repeat(size);
                let _ = manager.send_message(message).await;

                let _ = manager.disconnect().await;
                server.abort();
            });
        });
    }

    group.finish();
}

fn encryption_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("encryption_overhead");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let message_sizes = vec![64, 256, 1024, 4096];

    for size in message_sizes {
        group.bench_with_input(
            BenchmarkId::new("encrypted_message", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let encryption = AesGcmEncryption::new("test_password");
                    let message = "A".repeat(size);
                    let _ = encryption.encrypt("", &message).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_connections");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let connection_counts = vec![2, 4, 8, 16];

    for count in connection_counts {
        group.bench_with_input(
            BenchmarkId::new("connection_count", count),
            &count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    let mut servers = Vec::new();

                    for _ in 0..count {
                        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
                        let server = setup_test_server(addr).await;
                        servers.push(server);

                        let config = ConnectionConfig::new(addr);
                        let mut manager = ConnectionManager::new(config.clone());
                        let transport = TcpTransport::new(config);
                        let encryption = AesGcmEncryption::new("test_password");

                        manager.set_transport(Box::new(transport));
                        manager.set_encryption(Box::new(encryption));

                        let handle = tokio::spawn(async move {
                            let _ = manager.connect().await;
                            let _ = manager.disconnect().await;
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let _ = handle.await;
                    }

                    for server in servers {
                        server.abort();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    transport_benches,
    connection_establishment,
    message_roundtrip,
    encryption_overhead,
    concurrent_connections
);
criterion_main!(transport_benches);
