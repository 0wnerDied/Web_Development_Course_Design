use std::sync::Arc;

use axum::extract::FromRef;

use crate::health::Metrics;
use team_operation_system::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub metrics: Arc<Metrics>,
}

impl AppState {
    pub fn new(pool: DbPool, metrics: Arc<Metrics>) -> Self {
        Self { pool, metrics }
    }
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> DbPool {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Arc<Metrics> {
    fn from_ref(state: &AppState) -> Arc<Metrics> {
        state.metrics.clone()
    }
}
