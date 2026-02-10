//! HTTP metrics middleware.
//!
//! Records per-request counters and latency histograms.

use axum::{extract::MatchedPath, extract::Request, middleware::Next, response::Response};
use metrics::{counter, histogram};
use std::time::Instant;

/// Metrics middleware that records request count and duration.
///
/// Records:
/// - `http_requests_total` counter with labels: method, route, status
/// - `http_request_duration_seconds` histogram with labels: method, route, status
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed().as_secs_f64();

    let status = response.status().as_u16().to_string();

    counter!("http_requests_total",
        "method" => method.clone(),
        "route" => route.clone(),
        "status" => status.clone(),
    )
    .increment(1);

    histogram!("http_request_duration_seconds",
        "method" => method,
        "route" => route,
        "status" => status,
    )
    .record(duration);

    response
}
