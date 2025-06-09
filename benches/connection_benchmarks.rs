use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use std::net::SocketAddr;
use std::time::Duration;
use std::sync::Arc;

use lair_chat::client::{
    config::ConnectionConfig,
    connection_manager::ConnectionManager,
    tcp_transport::TcpTransport,
    aes_gcm_encryption::AesGcmEncryption,
};

async fn setup_echo_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 16384]; // 16KB buffer
                while let Ok(n) = socket.try_read(&mut buf) {
                    if n == 0 { break; }
                    if socket.try_write(&buf[..n]).is_err() { break; }
                }
            });
        }
    })
}

fn connection_recovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("connection_recovery");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    
    group.bench_function("recover_from_disconnect", |b| {
        b.to_async(&rt).iter(|| async {
            let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
            let server = setup_echo_server(addr).await;
            
            let config = ConnectionConfig::new(addr);
            let mut manager = ConnectionManager::new(config.clone());
            let transport = TcpTransport::new(config);
            let encryption = AesGcmEncryption::new("test_password");
            
            manager.set_transport(Box::new(transport));
            manager.set_encryption(Box::new(encryption));
            
            // Initial connection
            let _ = manager.connect().await;
            // Force disconnect
            let _ = manager.disconnect().await;
            // Attempt recovery
            let _ = manager.connect().await;
            let _ = manager.disconnect().await;
            
            server.abort();
        });
    });
    
    group.finish();
}

fn message_channel_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("message_channel_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    
    let message_counts = vec![100, 1000, 10000];
    
    for count in message_counts {
        group.bench_with_input(BenchmarkId::new("message_count", count), &count, |b, &count| {
            b.to_async(&rt).iter(|| async {
                let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
                let server = setup_echo_server(addr).await;
                
                let config = ConnectionConfig::new(addr);
                let mut manager = ConnectionManager::new(config.clone());
                let transport = TcpTransport::new(config);
                let encryption = AesGcmEncryption::new("test_password");
                
                manager.set_transport(Box::new(transport));
                manager.set_encryption(Box::new(encryption));
                
                let (tx, mut rx) = mpsc::channel(32);
                manager.register_message_channel(tx).await;
                
                let _ = manager.connect().await;
                
                for i in 0..count {
                    let message = format!("Message {}", i);
                    let _ = manager.send_message(message).await;
                }
                
                let _ = manager.disconnect().await;
                server.abort();
            });
        });
    }
    
    group.finish();
}

fn connection_status_transitions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("status_transitions");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    
    let transition_counts = vec![10, 50, 100];
    
    for count in transition_counts {
        group.bench_with_input(BenchmarkId::new("transition_count", count), &count, |b, &count| {
            b.to_async(&rt).iter(|| async {
                let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
                let server = setup_echo_server(addr).await;
                
                let config = ConnectionConfig::new(addr);
                let mut manager = ConnectionManager::new(config.clone());
                let transport = TcpTransport::new(config);
                let encryption = AesGcmEncryption::new("test_password");
                
                manager.set_transport(Box::new(transport));
                manager.set_encryption(Box::new(encryption));
                
                for _ in 0..count {
                    let _ = manager.connect().await;
                    let _ = manager.disconnect().await;
                }
                
                server.abort();
            });
        });
    }
    
    group.finish();
}

fn observer_notification(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("observer_notification");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    
    let observer_counts = vec![1, 5, 10];
    
    for count in observer_counts {
        group.bench_with_input(BenchmarkId::new("observer_count", count), &count, |b, &count| {
            b.to_async(&rt).iter(|| async {
                let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
                let server = setup_echo_server(addr).await;
                
                let config = ConnectionConfig::new(addr);
                let mut manager = ConnectionManager::new(config.clone());
                let transport = TcpTransport::new(config);
                let encryption = AesGcmEncryption::new("test_password");
                
                manager.set_transport(Box::new(transport));
                manager.set_encryption(Box::new(encryption));
                
                // Register multiple observers
                let mut channels = Vec::new();
                for _ in 0..count {
                    let (tx, _rx) = mpsc::channel(32);
                    manager.register_message_channel(tx).await;
                    channels.push(_rx);
                }
                
                // Perform actions that trigger notifications
                let _ = manager.connect().await;
                let _ = manager.send_message("Test message".to_string()).await;
                let _ = manager.disconnect().await;
                
                server.abort();
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    connection_benches,
    connection_recovery,
    message_channel_throughput,
    connection_status_transitions,
    observer_notification
);
criterion_main!(connection_benches);