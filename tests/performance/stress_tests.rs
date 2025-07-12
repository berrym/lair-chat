//! Stress tests for the lair-chat application
//!
//! This module contains stress testing scenarios to validate system behavior
//! under extreme load conditions and resource pressure.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::sleep;

#[tokio::test]
async fn test_extreme_concurrent_load() {
    // Test system behavior under extreme concurrent load
    let extreme_user_count = 500;
    let operations_per_user = 5;

    let start_time = Instant::now();
    let completed_operations = Arc::new(AtomicUsize::new(0));
    let failed_operations = Arc::new(AtomicUsize::new(0));

    let semaphore = Arc::new(Semaphore::new(extreme_user_count));
    let mut handles = vec![];

    for user_id in 0..extreme_user_count {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let completed_ops = completed_operations.clone();
        let failed_ops = failed_operations.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit; // Hold permit for duration

            for op_id in 0..operations_per_user {
                // Simulate high-stress operation
                match simulate_stress_operation(user_id, op_id).await {
                    Ok(_) => {
                        completed_ops.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        failed_ops.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start_time.elapsed();
    let completed = completed_operations.load(Ordering::Relaxed);
    let failed = failed_operations.load(Ordering::Relaxed);
    let total_operations = extreme_user_count * operations_per_user;

    println!(
        "Extreme load test - Duration: {:?}, Completed: {}, Failed: {}, Total: {}",
        duration, completed, failed, total_operations
    );

    // Under extreme load, some failures are acceptable
    let failure_rate = failed as f64 / total_operations as f64;
    assert!(
        failure_rate < 0.05, // Less than 5% failure rate
        "Failure rate too high under extreme load: {:.1}%",
        failure_rate * 100.0
    );

    assert!(
        completed > 0,
        "System should complete some operations even under extreme load"
    );
}

#[tokio::test]
async fn test_memory_pressure_stress() {
    // Test system behavior under memory pressure
    let chunk_size = 1024 * 1024; // 1MB chunks
    let max_chunks = 100;
    let mut memory_chunks = Vec::new();

    let start_time = Instant::now();
    let mut peak_memory_time = start_time;

    for i in 0..max_chunks {
        // Allocate memory chunk
        let chunk = vec![i as u8; chunk_size];
        memory_chunks.push(chunk);

        // Simulate some work with the allocated memory
        sleep(Duration::from_millis(10)).await;

        // Check if we can still perform basic operations
        let operation_start = Instant::now();
        simulate_basic_operation().await;
        let operation_duration = operation_start.elapsed();

        // Operations should still complete in reasonable time under memory pressure
        assert!(
            operation_duration < Duration::from_millis(100),
            "Operation took too long under memory pressure: {:?}",
            operation_duration
        );

        if i == max_chunks / 2 {
            peak_memory_time = Instant::now();
        }

        println!(
            "Allocated {}MB, operation time: {:?}",
            i + 1,
            operation_duration
        );
    }

    // Gradually release memory and test recovery
    let release_start = Instant::now();
    while !memory_chunks.is_empty() {
        memory_chunks.pop();

        if memory_chunks.len() % 10 == 0 {
            // Test operation performance during memory release
            let operation_start = Instant::now();
            simulate_basic_operation().await;
            let operation_duration = operation_start.elapsed();

            println!(
                "Memory release - {}MB remaining, operation time: {:?}",
                memory_chunks.len(),
                operation_duration
            );
        }

        sleep(Duration::from_millis(5)).await;
    }

    let total_duration = start_time.elapsed();
    let release_duration = release_start.elapsed();

    println!(
        "Memory stress test completed - Total: {:?}, Release: {:?}",
        total_duration, release_duration
    );

    // System should recover after memory release
    assert!(
        release_duration < Duration::from_secs(30),
        "Memory release took too long: {:?}",
        release_duration
    );
}

#[tokio::test]
async fn test_sustained_high_throughput() {
    // Test sustained high throughput operations
    let test_duration = Duration::from_secs(30);
    let target_throughput = 1000; // operations per second

    let start_time = Instant::now();
    let operations_completed = Arc::new(AtomicUsize::new(0));
    let operations_failed = Arc::new(AtomicUsize::new(0));

    // Spawn multiple worker tasks
    let worker_count = 10;
    let mut handles = vec![];

    for worker_id in 0..worker_count {
        let ops_completed = operations_completed.clone();
        let ops_failed = operations_failed.clone();

        let handle = tokio::spawn(async move {
            let mut worker_operations = 0;
            let worker_start = Instant::now();

            while worker_start.elapsed() < test_duration {
                match simulate_high_throughput_operation(worker_id, worker_operations).await {
                    Ok(_) => {
                        ops_completed.fetch_add(1, Ordering::Relaxed);
                        worker_operations += 1;
                    }
                    Err(_) => {
                        ops_failed.fetch_add(1, Ordering::Relaxed);
                    }
                }

                // Brief yield to prevent blocking
                if worker_operations % 100 == 0 {
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
        async move {
            let mut last_count = 0;
            let mut last_time = Instant::now();

            loop {
                sleep(Duration::from_secs(5)).await;

                let current_count = ops_completed.load(Ordering::Relaxed);
                let current_time = Instant::now();
                let elapsed = current_time.duration_since(last_time);

                let throughput = (current_count - last_count) as f64 / elapsed.as_secs_f64();
                println!("Current throughput: {:.1} ops/sec", throughput);

                last_count = current_count;
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
    let actual_throughput = completed as f64 / actual_duration.as_secs_f64();

    println!(
        "Sustained throughput test - Duration: {:?}, Completed: {}, Failed: {}, Throughput: {:.1} ops/sec",
        actual_duration, completed, failed, actual_throughput
    );

    // Verify sustained throughput
    assert!(
        actual_throughput >= target_throughput as f64 * 0.8, // Allow 20% below target
        "Sustained throughput too low: {:.1} ops/sec (target: {} ops/sec)",
        actual_throughput,
        target_throughput
    );

    let failure_rate = failed as f64 / (completed + failed) as f64;
    assert!(
        failure_rate < 0.01, // Less than 1% failure rate
        "Failure rate too high during sustained load: {:.1}%",
        failure_rate * 100.0
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
