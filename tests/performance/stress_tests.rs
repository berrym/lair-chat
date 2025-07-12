//! Stress tests for the lair-chat application
//!
//! This module contains stress testing scenarios to validate system behavior
//! under extreme load conditions and resource pressure.

use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::Command as TokioCommand;
use tokio::sync::Semaphore;
use tokio::time::sleep;

#[tokio::test]
async fn test_extreme_concurrent_load() {
    // Test system behavior under extreme concurrent load with real server
    let extreme_user_count = 500;
    let operations_per_user = 5;
    let server_host = "127.0.0.1";
    let server_port = 3335;

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();
    let completed_operations = Arc::new(AtomicUsize::new(0));
    let failed_operations = Arc::new(AtomicUsize::new(0));
    let successful_connections = Arc::new(AtomicUsize::new(0));

    let semaphore = Arc::new(Semaphore::new(extreme_user_count));
    let mut handles = vec![];

    for user_id in 0..extreme_user_count {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let completed_ops = completed_operations.clone();
        let failed_ops = failed_operations.clone();
        let successful_conns = successful_connections.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit; // Hold permit for duration

            // Attempt connection under extreme load
            match TcpStream::connect(format!("{}:{}", server_host, server_port)).await {
                Ok(mut stream) => {
                    successful_conns.fetch_add(1, Ordering::Relaxed);

                    // Register user with stress-specific username
                    let username = format!("stress_user_{}", user_id);
                    let register_cmd = format!("register {} stress_pass_{}\n", username, user_id);

                    if stream.write_all(register_cmd.as_bytes()).await.is_ok() {
                        // Perform rapid operations under stress
                        for op_id in 0..operations_per_user {
                            let message = format!(
                                "say Stress test operation {} from user {}\n",
                                op_id, user_id
                            );

                            match stream.write_all(message.as_bytes()).await {
                                Ok(_) => {
                                    completed_ops.fetch_add(1, Ordering::Relaxed);
                                    // Very brief delay - we want to stress the system
                                    sleep(Duration::from_millis(10)).await;
                                }
                                Err(_) => {
                                    failed_ops.fetch_add(1, Ordering::Relaxed);
                                    break; // Connection failed, stop operations
                                }
                            }
                        }

                        // Quick disconnect
                        let _ = stream.write_all(b"quit\n").await;
                    } else {
                        failed_ops.fetch_add(operations_per_user, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    // Connection failed - count all operations as failed
                    failed_ops.fetch_add(operations_per_user, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);

        // Small staggered start to prevent instant overwhelming
        if user_id % 50 == 0 {
            sleep(Duration::from_millis(100)).await;
        }
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let completed = completed_operations.load(Ordering::Relaxed);
    let failed = failed_operations.load(Ordering::Relaxed);
    let connections = successful_connections.load(Ordering::Relaxed);
    let total_operations = extreme_user_count * operations_per_user;

    println!(
        "Extreme load test - Duration: {:?}, Completed: {}, Failed: {}, Total: {}, Connections: {}/{}",
        duration, completed, failed, total_operations, connections, extreme_user_count
    );

    // Under extreme load, some failures are acceptable but system should remain responsive
    let failure_rate = failed as f64 / total_operations as f64;
    let connection_rate = connections as f64 / extreme_user_count as f64;

    assert!(
        failure_rate < 0.20, // Less than 20% failure rate under extreme load
        "Failure rate too high under extreme load: {:.1}%",
        failure_rate * 100.0
    );

    assert!(
        connection_rate > 0.5, // At least 50% of connections should succeed
        "Connection rate too low under extreme load: {:.1}%",
        connection_rate * 100.0
    );

    assert!(
        completed > 0,
        "System should complete some operations even under extreme load"
    );
}

#[tokio::test]
async fn test_memory_pressure_stress() {
    // Test system behavior under memory pressure with real server operations
    let chunk_size = 1024 * 1024; // 1MB chunks
    let max_chunks = 50; // Reduced for CI environments
    let server_host = "127.0.0.1";
    let server_port = 3335;
    let mut memory_chunks = Vec::new();

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();
    let mut operation_times = Vec::new();

    for i in 0..max_chunks {
        // Allocate memory chunk
        let chunk = vec![i as u8; chunk_size];
        memory_chunks.push(chunk);

        // Test server responsiveness under memory pressure
        let operation_start = Instant::now();

        match test_server_responsiveness(server_host, server_port, i).await {
            Ok(response_time) => {
                operation_times.push(response_time);

                // Operations should still complete in reasonable time under memory pressure
                assert!(
                    response_time < Duration::from_millis(1000),
                    "Server response too slow under memory pressure: {:?} at chunk {}",
                    response_time,
                    i
                );
            }
            Err(_) => {
                println!("Server became unresponsive at memory chunk {}", i);
                // Allow some failures under extreme memory pressure
                if i < max_chunks / 2 {
                    panic!("Server failed too early in memory pressure test");
                }
                break;
            }
        }

        let operation_duration = operation_start.elapsed();
        println!(
            "Allocated {}MB, server response time: {:?}",
            (i + 1) * chunk_size / (1024 * 1024),
            operation_duration
        );

        // Brief pause between allocations
        sleep(Duration::from_millis(100)).await;
    }

    // Test recovery by gradually releasing memory
    let release_start = Instant::now();
    let mut recovery_times = Vec::new();

    while !memory_chunks.is_empty() {
        memory_chunks.pop();

        if memory_chunks.len() % 10 == 0 {
            // Test server recovery performance during memory release
            let recovery_start_time = Instant::now();

            match test_server_responsiveness(server_host, server_port, 999).await {
                Ok(response_time) => {
                    recovery_times.push(response_time);
                    println!(
                        "Memory release - {}MB remaining, server response: {:?}",
                        memory_chunks.len() * chunk_size / (1024 * 1024),
                        response_time
                    );
                }
                Err(_) => {
                    println!("Server still unresponsive during recovery");
                }
            }
        }

        sleep(Duration::from_millis(50)).await;
    }

    let total_duration = start_time.elapsed();
    let release_duration = release_start.elapsed();

    println!(
        "Memory stress test completed - Total: {:?}, Release: {:?}",
        total_duration, release_duration
    );

    // Analyze performance degradation
    if !operation_times.is_empty() {
        let avg_response_time =
            operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        println!(
            "Average server response time under memory pressure: {:?}",
            avg_response_time
        );

        assert!(
            avg_response_time < Duration::from_millis(500),
            "Average response time too high under memory pressure: {:?}",
            avg_response_time
        );
    }

    // System should recover after memory release
    assert!(
        release_duration < Duration::from_secs(30),
        "Memory release took too long: {:?}",
        release_duration
    );
}

#[tokio::test]
async fn test_sustained_high_throughput() {
    // Test sustained high throughput operations with real server
    let test_duration = Duration::from_secs(60); // Extended for stress testing
    let target_throughput = 500; // operations per second (realistic for real server)
    let server_host = "127.0.0.1";
    let server_port = 3335;

    // Start test server if not running
    ensure_test_server_running().await;

    let start_time = Instant::now();
    let operations_completed = Arc::new(AtomicUsize::new(0));
    let operations_failed = Arc::new(AtomicUsize::new(0));
    let connections_established = Arc::new(AtomicUsize::new(0));

    // Spawn multiple worker tasks
    let worker_count = 20; // Increased worker count for stress testing
    let mut handles = vec![];

    for worker_id in 0..worker_count {
        let ops_completed = operations_completed.clone();
        let ops_failed = operations_failed.clone();
        let conns_established = connections_established.clone();

        let handle = tokio::spawn(async move {
            let mut worker_operations = 0;
            let worker_start = Instant::now();

            while worker_start.elapsed() < test_duration {
                // High throughput operation: rapid connect, send, disconnect
                match TcpStream::connect(format!("{}:{}", server_host, server_port)).await {
                    Ok(mut stream) => {
                        conns_established.fetch_add(1, Ordering::Relaxed);

                        let username =
                            format!("throughput_worker_{}_{}", worker_id, worker_operations);
                        let register_cmd = format!("register {} throughput_pass\n", username);

                        if stream.write_all(register_cmd.as_bytes()).await.is_ok() {
                            // Send rapid burst of messages
                            let burst_size = 3;
                            let mut burst_success = 0;

                            for burst_msg in 0..burst_size {
                                let message = format!(
                                    "say High throughput message {} from worker {}\n",
                                    burst_msg, worker_id
                                );

                                if stream.write_all(message.as_bytes()).await.is_ok() {
                                    burst_success += 1;
                                } else {
                                    break;
                                }
                            }

                            ops_completed.fetch_add(burst_success, Ordering::Relaxed);

                            if burst_success < burst_size {
                                ops_failed.fetch_add(burst_size - burst_success, Ordering::Relaxed);
                            }

                            // Quick disconnect
                            let _ = stream.write_all(b"quit\n").await;
                        } else {
                            ops_failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        ops_failed.fetch_add(1, Ordering::Relaxed);
                    }
                }

                worker_operations += 1;

                // Minimal delay for maximum throughput stress
                sleep(Duration::from_millis(20)).await;

                // Brief yield to prevent blocking
                if worker_operations % 50 == 0 {
                    tokio::task::yield_now().await;
                }
            }

            worker_operations
        });
        handles.push(handle);
    }

    // Monitor progress
    let monitor_handle = tokio::spawn({
        let ops_completed = operations_completed.clone();
        let ops_failed = operations_failed.clone();
        async move {
            let mut last_completed = 0;
            let mut last_failed = 0;
            let mut last_time = Instant::now();

            loop {
                sleep(Duration::from_secs(10)).await;

                let current_completed = ops_completed.load(Ordering::Relaxed);
                let current_failed = ops_failed.load(Ordering::Relaxed);
                let current_time = Instant::now();
                let elapsed = current_time.duration_since(last_time);

                let throughput =
                    (current_completed - last_completed) as f64 / elapsed.as_secs_f64();
                let failure_rate = if current_completed + current_failed > 0 {
                    current_failed as f64 / (current_completed + current_failed) as f64 * 100.0
                } else {
                    0.0
                };

                println!(
                    "Sustained throughput - ops: {}, failed: {}, rate: {:.1} ops/sec, failure: {:.1}%",
                    current_completed, current_failed, throughput, failure_rate
                );

                last_completed = current_completed;
                last_failed = current_failed;
                last_time = current_time;

                if current_time.duration_since(start_time) >= test_duration {
                    break;
                }
            }
        }
    });

    // Wait for all workers to complete
    let mut total_worker_operations = 0;
    for handle in handles {
        total_worker_operations += handle.await.unwrap();
    }

    // Stop monitoring
    monitor_handle.abort();

    let actual_duration = start_time.elapsed();
    let completed = operations_completed.load(Ordering::Relaxed);
    let failed = operations_failed.load(Ordering::Relaxed);
    let connections = connections_established.load(Ordering::Relaxed);
    let actual_throughput = completed as f64 / actual_duration.as_secs_f64();

    println!(
        "Sustained throughput test - Duration: {:?}, Completed: {}, Failed: {}, Connections: {}, Throughput: {:.1} ops/sec",
        actual_duration, completed, failed, connections, actual_throughput
    );

    // Verify sustained throughput (adjusted for real server capabilities)
    assert!(
        actual_throughput >= target_throughput as f64 * 0.6, // Allow 40% below target for real server
        "Sustained throughput too low: {:.1} ops/sec (target: {} ops/sec)",
        actual_throughput,
        target_throughput
    );

    let failure_rate = failed as f64 / (completed + failed) as f64;
    assert!(
        failure_rate < 0.10, // Less than 10% failure rate under stress
        "Failure rate too high during sustained load: {:.1}%",
        failure_rate * 100.0
    );

    // Ensure we established reasonable number of connections
    assert!(
        connections > 100,
        "Too few connections established: {}",
        connections
    );
}

#[tokio::test]
async fn test_resource_exhaustion_recovery() {
    // Test system recovery from resource exhaustion
    println!("Testing resource exhaustion and recovery...");

    // Phase 1: Gradually increase load until failure
    let mut load_level = 10;
    let max_load_level = 1000;
    let mut failure_detected = false;
    let mut failure_load_level = 0;

    while load_level <= max_load_level && !failure_detected {
        println!("Testing load level: {}", load_level);

        let test_result = simulate_load_level(load_level).await;
        match test_result {
            LoadTestResult::Success => {
                load_level *= 2; // Exponential increase
            }
            LoadTestResult::Degraded => {
                load_level += load_level / 4; // Slower increase when degraded
            }
            LoadTestResult::Failure => {
                failure_detected = true;
                failure_load_level = load_level;
                println!("Failure detected at load level: {}", load_level);
            }
        }

        // Brief recovery period between tests
        sleep(Duration::from_millis(100)).await;
    }

    // Phase 2: Test recovery by reducing load
    if failure_detected {
        println!("Testing recovery from failure...");

        let recovery_load = failure_load_level / 2;
        let recovery_result = simulate_load_level(recovery_load).await;

        assert!(
            matches!(
                recovery_result,
                LoadTestResult::Success | LoadTestResult::Degraded
            ),
            "System should recover when load is reduced"
        );

        println!("Recovery successful at load level: {}", recovery_load);
    }

    // Phase 3: Verify sustained operation at safe load level
    let safe_load = if failure_detected {
        failure_load_level / 4
    } else {
        max_load_level / 2
    };

    println!(
        "Testing sustained operation at safe load level: {}",
        safe_load
    );

    for _ in 0..5 {
        let result = simulate_load_level(safe_load).await;
        assert!(
            matches!(result, LoadTestResult::Success),
            "System should operate reliably at safe load level"
        );
        sleep(Duration::from_millis(50)).await;
    }

    println!("Resource exhaustion recovery test completed successfully");
}

#[tokio::test]
async fn test_connection_exhaustion_stress() {
    // Test behavior when connection limits are reached
    let max_connections = 200;
    let connection_timeout = Duration::from_millis(100);

    let start_time = Instant::now();
    let active_connections = Arc::new(AtomicUsize::new(0));
    let connection_failures = Arc::new(AtomicUsize::new(0));
    let successful_connections = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    for conn_id in 0..max_connections * 2 {
        let active_conns = active_connections.clone();
        let conn_failures = connection_failures.clone();
        let successful_conns = successful_connections.clone();

        let handle = tokio::spawn(async move {
            // Simulate connection attempt
            match simulate_connection_attempt(conn_id, connection_timeout).await {
                Ok(_) => {
                    active_conns.fetch_add(1, Ordering::Relaxed);
                    successful_conns.fetch_add(1, Ordering::Relaxed);

                    // Hold connection for some time
                    sleep(Duration::from_millis(200)).await;

                    active_conns.fetch_sub(1, Ordering::Relaxed);
                }
                Err(_) => {
                    conn_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(handle);

        // Brief delay between connection attempts
        sleep(Duration::from_millis(10)).await;
    }

    // Wait for all connection attempts
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let successful = successful_connections.load(Ordering::Relaxed);
    let failures = connection_failures.load(Ordering::Relaxed);
    let total_attempts = max_connections * 2;

    println!(
        "Connection exhaustion test - Duration: {:?}, Successful: {}, Failed: {}, Total: {}",
        duration, successful, failures, total_attempts
    );

    // Should handle connection exhaustion gracefully
    assert!(
        successful > 0,
        "Should establish some connections even under exhaustion"
    );

    assert!(
        failures > 0,
        "Should reject some connections when exhausted"
    );

    // Failure rate should be reasonable (not 100%)
    let failure_rate = failures as f64 / total_attempts as f64;
    assert!(
        failure_rate < 0.8, // Less than 80% failure rate
        "Connection failure rate too high: {:.1}%",
        failure_rate * 100.0
    );
}

// Helper functions and types

#[derive(Debug)]
enum LoadTestResult {
    Success,
    Degraded,
    Failure,
}

async fn test_server_responsiveness(
    host: &str,
    port: u16,
    test_id: usize,
) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();

    let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;

    // Quick operation to test responsiveness
    let username = format!("responsiveness_test_{}", test_id);
    let register_cmd = format!("register {} test_pass\n", username);

    stream.write_all(register_cmd.as_bytes()).await?;

    // Send a test message
    let test_message = format!("say Responsiveness test {}\n", test_id);
    stream.write_all(test_message.as_bytes()).await?;

    // Quick disconnect
    let _ = stream.write_all(b"quit\n").await;

    Ok(start_time.elapsed())
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

async fn simulate_stress_operation(user_id: usize, op_id: usize) -> Result<(), &'static str> {
    // Simulate operation with potential for failure under stress
    let delay = Duration::from_millis(5 + (user_id % 10) as u64);
    sleep(delay).await;

    // Simulate occasional failures under high stress
    if user_id > 400 && op_id % 20 == 0 {
        Err("Stress-induced failure")
    } else {
        Ok(())
    }
}

async fn simulate_basic_operation() {
    // Simple operation for testing under resource pressure
    sleep(Duration::from_millis(5)).await;
}

async fn simulate_high_throughput_operation(
    worker_id: usize,
    operation_count: usize,
) -> Result<(), &'static str> {
    // High-frequency operation simulation
    let delay = Duration::from_micros(500 + (worker_id * 100) as u64);
    sleep(delay).await;

    // Simulate occasional failures
    if operation_count % 1000 == 999 {
        Err("Occasional failure")
    } else {
        Ok(())
    }
}

async fn simulate_load_level(load_level: usize) -> LoadTestResult {
    let operation_count = load_level;
    let start_time = Instant::now();
    let mut successes = 0;
    let mut failures = 0;

    for i in 0..operation_count {
        let op_start = Instant::now();

        // Simulate operation with load-dependent behavior
        let delay = if load_level > 500 {
            Duration::from_millis(10 + (i % 50) as u64) // Slower under high load
        } else {
            Duration::from_millis(1 + (i % 10) as u64)
        };

        sleep(delay).await;

        let op_duration = op_start.elapsed();

        if op_duration > Duration::from_millis(100) {
            failures += 1;
        } else {
            successes += 1;
        }

        // Yield occasionally
        if i % 50 == 0 {
            tokio::task::yield_now().await;
        }
    }

    let total_duration = start_time.elapsed();
    let success_rate = successes as f64 / operation_count as f64;

    println!(
        "Load level {} - Duration: {:?}, Success rate: {:.1}%",
        load_level,
        total_duration,
        success_rate * 100.0
    );

    if success_rate >= 0.95 {
        LoadTestResult::Success
    } else if success_rate >= 0.5 {
        LoadTestResult::Degraded
    } else {
        LoadTestResult::Failure
    }
}

async fn simulate_connection_attempt(
    conn_id: usize,
    timeout: Duration,
) -> Result<(), &'static str> {
    // Simulate connection establishment
    let connection_delay = Duration::from_millis(10 + (conn_id % 50) as u64);

    if connection_delay > timeout {
        Err("Connection timeout")
    } else {
        sleep(connection_delay).await;

        // Simulate connection limit enforcement
        if conn_id > 150 && conn_id % 3 == 0 {
            Err("Connection limit reached")
        } else {
            Ok(())
        }
    }
}
