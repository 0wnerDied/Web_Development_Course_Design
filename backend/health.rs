use axum::{extract::State, http::StatusCode, Json};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use team_operation_system::db::DbPool;

/// 系统指标
pub struct Metrics {
    pub request_count: AtomicU64,
    pub error_count: AtomicU64,
    pub db_query_count: AtomicU64,
    pub start_time: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            db_query_count: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn increment_requests(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_db_queries(&self) {
        self.db_query_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub database: DatabaseStatus,
    pub metrics: MetricsSnapshot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStatus {
    pub connected: bool,
    pub response_time_ms: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_db_queries: u64,
    pub uptime_seconds: u64,
    pub error_rate: f64,
}

/// 健康检查端点
pub async fn health_check(
    State(pool): State<DbPool>,
    State(metrics): State<Arc<Metrics>>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let start = Instant::now();

    // 检查数据库连接
    let db_health = match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => {
            let response_time = start.elapsed().as_secs_f64() * 1000.0;
            DatabaseStatus {
                connected: true,
                response_time_ms: Some(response_time),
            }
        }
        Err(e) => {
            log::error!("Database health check failed: {}", e);
            DatabaseStatus {
                connected: false,
                response_time_ms: None,
            }
        }
    };

    // 获取指标快照
    let total_requests = metrics.request_count.load(Ordering::Relaxed);
    let total_errors = metrics.error_count.load(Ordering::Relaxed);
    let error_rate = if total_requests > 0 {
        (total_errors as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    let metrics_snapshot = MetricsSnapshot {
        total_requests,
        total_errors,
        total_db_queries: metrics.db_query_count.load(Ordering::Relaxed),
        uptime_seconds: metrics.get_uptime().as_secs(),
        error_rate,
    };

    let status = if db_health.connected {
        "healthy"
    } else {
        "unhealthy"
    };

    let db_connected = db_health.connected;
    let response = HealthResponse {
        status: status.to_string(),
        timestamp: Local::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_health,
        metrics: metrics_snapshot,
    };

    if db_connected {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// 详细指标端点
pub async fn metrics_endpoint(
    State(pool): State<DbPool>,
    State(metrics): State<Arc<Metrics>>,
) -> Result<Json<Value>, StatusCode> {
    let total_requests = metrics.request_count.load(Ordering::Relaxed);
    let total_errors = metrics.error_count.load(Ordering::Relaxed);
    let total_db_queries = metrics.db_query_count.load(Ordering::Relaxed);
    let uptime = metrics.get_uptime();

    // 获取数据库统计信息
    let db_stats = get_database_stats(&pool).await.ok();

    Ok(Json(json!({
        "timestamp": Local::now().to_rfc3339(),
        "uptime_seconds": uptime.as_secs(),
        "uptime_human": format_duration(uptime),
        "requests": {
            "total": total_requests,
            "errors": total_errors,
            "success": total_requests.saturating_sub(total_errors),
            "error_rate_percent": if total_requests > 0 {
                (total_errors as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            }
        },
        "database": {
            "total_queries": total_db_queries,
            "pool_size": pool.size(),
            "idle_connections": pool.num_idle(),
            "stats": db_stats
        }
    })))
}

/// 获取数据库统计信息
async fn get_database_stats(pool: &SqlitePool) -> Result<Value, sqlx::Error> {
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user")
        .fetch_one(pool)
        .await?;

    let lp_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lplog")
        .fetch_one(pool)
        .await?;

    let permission_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM permission")
        .fetch_one(pool)
        .await?;

    let log_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM requestlog")
        .fetch_one(pool)
        .await?;

    Ok(json!({
        "users": user_count.0,
        "lp_logs": lp_count.0,
        "permissions": permission_count.0,
        "request_logs": log_count.0
    }))
}

/// 格式化持续时间
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Prometheus 格式的指标端点
pub async fn prometheus_metrics(State(metrics): State<Arc<Metrics>>) -> Result<String, StatusCode> {
    let total_requests = metrics.request_count.load(Ordering::Relaxed);
    let total_errors = metrics.error_count.load(Ordering::Relaxed);
    let total_db_queries = metrics.db_query_count.load(Ordering::Relaxed);
    let uptime = metrics.get_uptime().as_secs();

    let prometheus_output = format!(
        r#"# HELP personnel_system_requests_total Total number of requests
# TYPE personnel_system_requests_total counter
personnel_system_requests_total {}

# HELP personnel_system_errors_total Total number of errors
# TYPE personnel_system_errors_total counter
personnel_system_errors_total {}

# HELP personnel_system_db_queries_total Total number of database queries
# TYPE personnel_system_db_queries_total counter
personnel_system_db_queries_total {}

# HELP personnel_system_uptime_seconds System uptime in seconds
# TYPE personnel_system_uptime_seconds gauge
personnel_system_uptime_seconds {}
"#,
        total_requests, total_errors, total_db_queries, uptime
    );

    Ok(prometheus_output)
}
