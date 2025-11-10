use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

use crate::health::Metrics;

/// 请求监控中间件
pub async fn metrics_middleware(
    State(metrics): State<Arc<Metrics>>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();
    let method = request.method().to_string();

    // 记录请求开始
    metrics.increment_requests();
    log::info!("Request started: {} {}", method, path);

    // 执行请求
    let response = next.run(request).await;

    // 记录请求完成
    let duration = start.elapsed();
    let status = response.status();

    // 如果是错误状态码，增加错误计数
    if status.is_client_error() || status.is_server_error() {
        metrics.increment_errors();
        log::warn!(
            "Request failed: {} {} - Status: {} - Duration: {:?}",
            method,
            path,
            status.as_u16(),
            duration
        );
    } else {
        log::info!(
            "Request completed: {} {} - Status: {} - Duration: {:?}",
            method,
            path,
            status.as_u16(),
            duration
        );
    }

    response
}

/// 请求日志中间件
pub async fn request_logging_middleware(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    let start = Instant::now();

    log::debug!("→ {} {}", method, path);

    let response = next.run(request).await;
    let duration = start.elapsed();

    log::debug!(
        "← {} {} - {} ({:?})",
        method,
        path,
        response.status().as_u16(),
        duration
    );

    response
}
