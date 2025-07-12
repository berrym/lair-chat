//! Load tests for the lair-chat application
//!
//! This module contains load testing scenarios to validate system performance
//! under expected production load conditions.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::Command as TokioCommand;
use tokio::sync::Semaphore;
use tokio::time::sleep;

#[tokio::test]
async fn test_concurrent_user_load() {
    // Test system behavior with multiple concurrent users
    let concurrent_users = 50;
    let messages_per_user = 10;
    let server_host = "127.0.0.1";
    let server_port = 3335;

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrent_users));
    let mut handles = vec![];
    let successful_connections = Arc::new(AtomicUsize::new(0));
    let successful_messages = Arc::new(AtomicUsize::new(0));
    let failed_operations = Arc::new(AtomicUsize::new(0));

    for user_id in 0..concurrent_users {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let successful_conns = successful_connections.clone();
        let successful_msgs = successful_messages.clone();
        let failed_ops = failed_operations.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit; // Hold permit for duration of test

            // Attempt real TCP connection to server
            match TcpStream::connect(format!("{}:{}", server_host, server_port)).await {
                Ok(mut stream) => {
                    successful_conns.fetch_add(1, Ordering::Relaxed);

                    // Register user
                    let username = format!("loadtest_user_{}", user_id);
                    let register_cmd = format!("register {} loadtest_pass\n", username);

                    if stream.write_all(register_cmd.as_bytes()).await.is_ok() {
                        // Send messages
                        for msg_id in 0..messages_per_user {
                            let message =
                                format!("say Load test message {} from user {}\n", msg_id, user_id);

                            if stream.write_all(message.as_bytes()).await.is_ok() {
                                successful_msgs.fetch_add(1, Ordering::Relaxed);
                                sleep(Duration::from_millis(100)).await;
                            } else {
                                failed_ops.fetch_add(1, Ordering::Relaxed);
                                break;
                            }
                        }

                        // Graceful disconnect
                        let _ = stream.write_all(b"quit\n").await;
                    } else {
                        failed_ops.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    failed_ops.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all users to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let conn_count = successful_connections.load(Ordering::Relaxed);
    let msg_count = successful_messages.load(Ordering::Relaxed);
    let fail_count = failed_operations.load(Ordering::Relaxed);

    println!("Load test completed in {:?}", duration);
    println!(
        "Successful connections: {}/{}",
        conn_count, concurrent_users
    );
    println!("Successful messages: {}", msg_count);
    println!("Failed operations: {}", fail_count);

    // Verify performance criteria
    assert!(
        duration < Duration::from_secs(30),
        "Load test should complete within 30 seconds"
    );

    assert!(
        conn_count >= concurrent_users * 9 / 10, // At least 90% success rate
        "Connection success rate too low: {}/{}",
        conn_count,
        concurrent_users
    );
}

#[tokio::test]
async fn test_message_throughput() {
    // Test message processing throughput with real server
    let message_count = 1000;
    let server_host = "127.0.0.1";
    let server_port = 3335;

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();

    // Establish connection
    let mut stream = TcpStream::connect(format!("{}:{}", server_host, server_port))
        .await
        .expect("Failed to connect to test server");

    // Register user
    let register_cmd = "register throughput_test_user test_password\n";
    stream.write_all(register_cmd.as_bytes()).await.unwrap();

    // Brief wait for registration
    sleep(Duration::from_millis(100)).await;

    let mut successful_messages = 0;
    let mut failed_messages = 0;

    for i in 0..message_count {
        let message = format!("say Throughput test message {}\n", i);

        match stream.write_all(message.as_bytes()).await {
            Ok(_) => {
                successful_messages += 1;
                if i % 100 == 0 {
                    println!("Sent {} messages", i);
                }
            }
            Err(_) => {
                failed_messages += 1;
            }
        }

        // Small delay to prevent overwhelming
        sleep(Duration::from_micros(500)).await;
    }

    let duration = start_time.elapsed();
    let throughput = successful_messages as f64 / duration.as_secs_f64();

    println!("Message throughput: {:.2} messages/second", throughput);
    println!(
        "Successful messages: {}/{}",
        successful_messages, message_count
    );
    println!("Failed messages: {}", failed_messages);

    // Graceful disconnect
    let _ = stream.write_all(b"quit\n").await;

    // Verify minimum throughput
    assert!(
        throughput > 100.0,
        "Should process at least 100 messages per second, got {:.2}",
        throughput
    );

    assert!(
        failed_messages < message_count / 10, // Less than 10% failures
        "Too many failed messages: {}/{}",
        failed_messages,
        message_count
    );
}

#[tokio::test]
async fn test_connection_establishment_load() {
    // Test rapid connection establishment with real server
    let connection_count = 100;
    let server_host = "127.0.0.1";
    let server_port = 3335;

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();
    let successful_connections = Arc::new(AtomicUsize::new(0));
    let failed_connections = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for conn_id in 0..connection_count {
        let successful_conns = successful_connections.clone();
        let failed_conns = failed_connections.clone();

        let handle = tokio::spawn(async move {
            let conn_start = Instant::now();

            match TcpStream::connect(format!("{}:{}", server_host, server_port)).await {
                Ok(mut stream) => {
                    let conn_time = conn_start.elapsed();
                    successful_conns.fetch_add(1, Ordering::Relaxed);

                    // Quick registration test
                    let username = format!("conn_test_user_{}", conn_id);
                    let register_cmd = format!("register {} test_pass\n", username);

                    if stream.write_all(register_cmd.as_bytes()).await.is_ok() {
                        // Immediate disconnect
                        let _ = stream.write_all(b"quit\n").await;
                    }

                    println!("Connection {} established in {:?}", conn_id, conn_time);
                }
                Err(e) => {
                    failed_conns.fetch_add(1, Ordering::Relaxed);
                    println!("Connection {} failed: {}", conn_id, e);
                }
            }
        });
        handles.push(handle);

        // Brief delay to prevent overwhelming the server
        sleep(Duration::from_millis(5)).await;
    }

    // Wait for all connections
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let successful_count = successful_connections.load(Ordering::Relaxed);
    let failed_count = failed_connections.load(Ordering::Relaxed);
    let connection_rate = successful_count as f64 / duration.as_secs_f64();

    println!("Connection rate: {:.2} connections/second", connection_rate);
    println!(
        "Successful connections: {}/{}",
        successful_count, connection_count
    );
    println!("Failed connections: {}", failed_count);

    // Verify minimum connection rate
    assert!(
        connection_rate > 50.0,
        "Should establish at least 50 connections per second, got {:.2}",
        connection_rate
    );

    assert!(
        successful_count >= connection_count * 9 / 10, // At least 90% success rate
        "Connection success rate too low: {}/{}",
        successful_count,
        connection_count
    );
}

#[tokio::test]
async fn test_sustained_load() {
    // Test system behavior under sustained load with real server
    let test_duration = Duration::from_secs(60); // Extended test
    let server_host = "127.0.0.1";
    let server_port = 3335;
    let concurrent_operations = 10;

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();
    let operation_count = Arc::new(AtomicUsize::new(0));
    let successful_operations = Arc::new(AtomicUsize::new(0));
    let failed_operations = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Start multiple sustained operation workers
    for worker_id in 0..concurrent_operations {
        let ops_count = operation_count.clone();
        let successful_ops = successful_operations.clone();
        let failed_ops = failed_operations.clone();

        let handle = tokio::spawn(async move {
            let mut worker_operations = 0;
            let worker_start = Instant::now();

            while worker_start.elapsed() < test_duration {
                match TcpStream::connect(format!("{}:{}", server_host, server_port)).await {
                    Ok(mut stream) => {
                        // Quick operation: register, send message, disconnect
                        let username =
                            format!("sustained_user_{}_{}", worker_id, worker_operations);
                        let register_cmd = format!("register {} test_pass\n", username);

                        if stream.write_all(register_cmd.as_bytes()).await.is_ok() {
                            let message =
                                format!("say Sustained test message {}\n", worker_operations);
                            if stream.write_all(message.as_bytes()).await.is_ok() {
                                successful_ops.fetch_add(1, Ordering::Relaxed);
                            } else {
                                failed_ops.fetch_add(1, Ordering::Relaxed);
                            }
                            let _ = stream.write_all(b"quit\n").await;
                        } else {
                            failed_ops.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        failed_ops.fetch_add(1, Ordering::Relaxed);
                    }
                }

                worker_operations += 1;
                ops_count.fetch_add(1, Ordering::Relaxed);

                // Brief delay between operations
                sleep(Duration::from_millis(100)).await;
            }

            worker_operations
        });
        handles.push(handle);
    }

    // Monitor progress
    let monitor_handle = tokio::spawn({
        let ops_count = operation_count.clone();
        async move {
            let mut last_count = 0;
            let mut last_time = Instant::now();

            loop {
                sleep(Duration::from_secs(10)).await;

                let current_count = ops_count.load(Ordering::Relaxed);
                let current_time = Instant::now();
                let elapsed = current_time.duration_since(last_time);

                let rate = (current_count - last_count) as f64 / elapsed.as_secs_f64();
                println!(
                    "Sustained load - operations: {}, rate: {:.2} ops/sec",
                    current_count, rate
                );

                last_count = current_count;
                last_time = current_time;

                if current_time.duration_since(start_time) >= test_duration {
                    break;
                }
            }
        }
    });

    // Wait for all workers to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Stop monitoring
    monitor_handle.abort();

    let actual_duration = start_time.elapsed();
    let total_operations = operation_count.load(Ordering::Relaxed);
    let successful_count = successful_operations.load(Ordering::Relaxed);
    let failed_count = failed_operations.load(Ordering::Relaxed);
    let operation_rate = successful_count as f64 / actual_duration.as_secs_f64();

    println!("Sustained operation rate: {:.2} ops/second", operation_rate);
    println!("Total operations: {}", total_operations);
    println!("Successful operations: {}", successful_count);
    println!("Failed operations: {}", failed_count);

    // Verify sustained performance
    assert!(
        operation_rate > 50.0,
        "Should maintain at least 50 operations per second, got {:.2}",
        operation_rate
    );

    let failure_rate = failed_count as f64 / total_operations as f64;
    assert!(
        failure_rate < 0.05, // Less than 5% failure rate
        "Failure rate too high: {:.1}%",
        failure_rate * 100.0
    );
}

// Helper function to ensure test server is running
async fn ensure_test_server_running() {
    let server_host = "127.0.0.1";
    let server_port = 3335;

    // Check if server is already running
    match TcpStream::connect(format!("{}:{}", server_host, server_port)).await {
        Ok(_) => {
            println!("Test server is already running");
            return;
        }
        Err(_) => {
            println!("Starting test server...");
        }
    }

    // Start server in background (this would typically be done externally)
    let server_process = TokioCommand::new("cargo")
        .args(&[
            "run",
            "--bin",
            "lair-chat-server",
            "--",
            "--config",
            "config/test.toml",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    if server_process.is_ok() {
        // Wait for server to start
        for _ in 0..30 {
            sleep(Duration::from_millis(500)).await;
            if TcpStream::connect(format!("{}:{}", server_host, server_port))
                .await
                .is_ok()
            {
                println!("Test server started successfully");
                return;
            }
        }
        println!("Warning: Test server may not have started properly");
    } else {
        println!("Warning: Could not start test server automatically");
    }
}
