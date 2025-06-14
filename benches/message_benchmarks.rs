use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use lair_chat::client::{
    aes_gcm_encryption::AesGcmEncryption, config::ConnectionConfig,
    connection_manager::ConnectionManager, tcp_transport::TcpTransport,
};

// Helper function to generate random messages of specified size
fn generate_random_message(size: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = thread_rng();
    (0..size)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

async fn setup_test_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
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

fn message_batching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_batching");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let batch_sizes = vec![1, 10, 50, 100];
    let message_size = 1024; // 1KB messages

    for size in batch_sizes {
        group.bench_with_input(BenchmarkId::new("batch_size", size), &size, |b, &size| {
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

                let message = generate_random_message(message_size);
                let mut futures = Vec::with_capacity(size);

                for _ in 0..size {
                    futures.push(manager.send_message(message.clone()));
                }

                futures::future::join_all(futures).await;

                let _ = manager.disconnect().await;
                server.abort();
            });
        });
    }

    group.finish();
}

fn message_size_impact(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_size_impact");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let message_sizes = vec![64, 256, 1024, 4096, 16384, 65536]; // From 64B to 64KB

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

                let message = generate_random_message(size);
                let _ = manager.send_message(message).await;

                let _ = manager.disconnect().await;
                server.abort();
            });
        });
    }

    group.finish();
}

fn message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let durations = vec![Duration::from_secs(1), Duration::from_secs(5)];

    for duration in durations {
        group.bench_with_input(
            BenchmarkId::new("duration_secs", duration.as_secs()),
            &duration,
            |b, duration| {
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

                    let message = generate_random_message(1024);
                    let start = std::time::Instant::now();
                    let mut count = 0;

                    while start.elapsed() < *duration {
                        let _ = manager.send_message(message.clone()).await;
                        count += 1;
                    }

                    let _ = manager.disconnect().await;
                    server.abort();
                });
            },
        );
    }

    group.finish();
}

fn concurrent_message_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_message_processing");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let concurrent_counts = vec![2, 4, 8, 16];

    for count in concurrent_counts {
        group.bench_with_input(
            BenchmarkId::new("concurrent_count", count),
            &count,
            |b, &count| {
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

                    let message = generate_random_message(1024);
                    let mut futures = Vec::with_capacity(count);

                    for _ in 0..count {
                        let msg = message.clone();
                        futures.push(tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 100))
                                .await;
                            msg
                        }));
                    }

                    for future in futures {
                        let msg = future.await.unwrap();
                        let _ = manager.send_message(msg).await;
                    }

                    let _ = manager.disconnect().await;
                    server.abort();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    message_benches,
    message_batching,
    message_size_impact,
    message_throughput,
    concurrent_message_processing
);
criterion_main!(message_benches);
