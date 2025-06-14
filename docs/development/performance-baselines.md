# Performance Baselines for Lair-Chat

## Overview

This document establishes the initial performance baselines for the Lair-Chat transport system. These metrics serve as reference points for monitoring performance regressions and improvements.

## Transport Layer Metrics

### Connection Performance

| Metric | Expected Range | Warning Threshold | Critical Threshold |
|--------|---------------|-------------------|-------------------|
| Connection Establishment | 50-150ms | >200ms | >500ms |
| Handshake Completion | 100-250ms | >300ms | >750ms |
| Disconnect Time | 10-50ms | >100ms | >250ms |
| Connection Recovery | 150-400ms | >500ms | >1000ms |

### Message Handling

| Message Size | Expected Throughput | Latency (Round Trip) |
|-------------|---------------------|---------------------|
| 64B | 5000+ msgs/sec | 1-5ms |
| 1KB | 2500+ msgs/sec | 2-10ms |
| 16KB | 500+ msgs/sec | 5-20ms |
| 64KB | 150+ msgs/sec | 10-50ms |

### Encryption Performance

| Operation | Expected Time | Warning Threshold |
|-----------|--------------|-------------------|
| Key Generation | 1-5ms | >10ms |
| Message Encryption (1KB) | 0.1-0.5ms | >1ms |
| Message Decryption (1KB) | 0.1-0.5ms | >1ms |
| Handshake | 50-150ms | >200ms |

### Concurrent Operations

| Scenario | Expected Performance |
|----------|---------------------|
| Concurrent Connections (10) | <500ms total establishment |
| Message Batching (100 msgs) | <1000ms total processing |
| Parallel Message Processing | 80%+ of single-message throughput |
| Observer Notifications (5 observers) | <1ms additional latency |

## System Resource Usage

### Memory Consumption

| Component | Base Usage | Per-Connection | Per-Message (In-Flight) |
|-----------|------------|----------------|------------------------|
| Server | 10-20MB | +500KB | +message size |
| Client | 5-10MB | +250KB | +message size |
| Connection Manager | 1-2MB | +100KB | negligible |

### CPU Usage

| Operation | Expected CPU Usage |
|-----------|-------------------|
| Idle Connected | <1% |
| Active Messaging | 5-15% |
| Peak Load | <30% |
| Encryption Operations | 10-20% during handshake |

## Network Usage

### Bandwidth Requirements

| Scenario | Expected Bandwidth |
|----------|-------------------|
| Idle Connection | <1 KB/s |
| Normal Usage | 10-50 KB/s |
| Heavy Usage | 100-500 KB/s |
| Peak Usage | <1 MB/s |

### Connection Overhead

| Metric | Size |
|--------|------|
| Connection Establishment | 1-2KB |
| Handshake Data | 2-3KB |
| Message Overhead | 40-60 bytes |
| Keepalive | 20-30 bytes/minute |

## Performance Under Load

### Stress Test Results

| Test Scenario | Expected Performance |
|--------------|---------------------|
| 100 concurrent connections | <5% message loss |
| 1000 msgs/sec for 1 hour | <1% message loss |
| Network latency >100ms | <10% throughput reduction |
| 50% packet loss | Auto-recovery <5s |

### Resource Limits

| Resource | Recommended Limit |
|----------|------------------|
| Max Connections (Server) | 1000 |
| Max Message Size | 1MB |
| Message Queue Size | 1000 messages |
| Backoff Timer | 50ms - 5s |

## Monitoring Thresholds

### Critical Alerts

| Metric | Warning | Critical |
|--------|---------|----------|
| Message Queue Length | >500 | >900 |
| Message Latency | >100ms | >1s |
| Connection Failures | >5/minute | >20/minute |
| Memory Usage | >80% | >90% |

### Performance Degradation Indicators

- Message throughput drops below 50% of baseline
- Connection establishment time increases by >100%
- Error rate exceeds 1% of messages
- CPU usage remains above 50% for >5 minutes

## Testing Environment

### Benchmark Configuration

- CPU: 4 cores @ 2.5GHz
- Memory: 8GB RAM
- Network: <1ms latency, 1Gbps bandwidth
- OS: Linux/macOS
- Rust: Latest stable release

### Test Parameters

- Message sizes: 64B, 1KB, 16KB, 64KB
- Concurrent connections: 1, 10, 100
- Duration: 60 seconds per test
- Samples: 50 iterations minimum

## Performance Improvement Targets

### Short-term Goals

1. Reduce connection establishment time by 20%
2. Increase message throughput by 30%
3. Reduce memory usage per connection by 25%
4. Improve encryption performance by 15%

### Long-term Goals

1. Support 10,000 concurrent connections
2. Achieve sub-millisecond message latency
3. Reduce CPU usage by 40%
4. Implement automatic performance scaling

## Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2023-12 | 1.0 | Initial baseline measurements |

## Notes

1. All measurements are taken on the reference testing environment
2. Network conditions may significantly impact actual performance
3. Performance may vary based on hardware and OS configuration
4. Encryption overhead increases with message size
5. Regular benchmark runs are scheduled weekly

## Next Steps

1. Implement continuous performance monitoring
2. Add detailed logging for performance metrics
3. Create automated performance regression tests
4. Establish production monitoring dashboards