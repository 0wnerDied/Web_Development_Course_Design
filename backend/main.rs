#![allow(non_snake_case)]

mod api;
mod auth;
mod health;
mod middleware;
mod scheduler;
mod state;

use axum::{middleware as axum_middleware, routing::get, Router};
use log::info;
use sqlx::Executor;
use std::net::SocketAddr;
use std::sync::Arc;
use team_operation_system::db;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // 初始化日志和追踪
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("团队运营管理系统后端启动中...");

    // 初始化数据库连接池（启用外键约束）
    let database_url = "sqlite:team.db?mode=rwc";
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute("PRAGMA foreign_keys = ON;").await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .expect("无法连接到数据库");

    db::init_database(&pool).await.expect("数据库初始化失败");
    info!("数据库初始化完成（外键约束已启用）");

    // 启动抽奖定时任务
    scheduler::start_lottery_scheduler(pool.clone()).await;
    info!("抽奖定时任务已启动（每分钟检查一次）");

    // 创建指标收集器
    let metrics = Arc::new(health::Metrics::new());
    info!("指标收集器初始化完成");

    let app_state = state::AppState::new(pool.clone(), metrics.clone());
    info!("应用状态初始化完成");

    // 配置 CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 构建 API 路由
    let api_routes = api::routes().layer(axum_middleware::from_fn_with_state(
        metrics.clone(),
        middleware::metrics_middleware,
    ));

    // 组合所有路由
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health::health_check))
        .route("/metrics", get(health::metrics_endpoint))
        .route("/metrics/prometheus", get(health::prometheus_metrics))
        .nest("/api", api_routes)
        .layer(axum_middleware::from_fn(
            middleware::request_logging_middleware,
        ))
        .layer(cors)
        .with_state(app_state);

    // 启动服务器
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("后端服务运行在 http://{}", addr);
    info!("健康检查: http://{}/health", addr);
    info!("指标监控: http://{}/metrics", addr);
    info!("Prometheus: http://{}/metrics/prometheus", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root_handler() -> &'static str {
    "团队运营管理系统后端服务运行中 | API: /api | 健康检查: /health | 指标: /metrics"
}
