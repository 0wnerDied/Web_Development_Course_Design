use crate::db::DbResult;
use crate::models::RequestLog;
use sqlx::SqlitePool;

pub async fn list_request_logs(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<RequestLog>> {
    let logs = sqlx::query_as::<_, RequestLog>(
        "SELECT id, method, path, user_qq, body, status, timestamp
         FROM requestlog
         ORDER BY timestamp DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(logs)
}

pub async fn count_request_logs(pool: &SqlitePool) -> DbResult<i64> {
    let (count,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM requestlog")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn count_request_logs_by_user(pool: &SqlitePool, user_qq: &str) -> DbResult<i64> {
    let (count,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM requestlog WHERE user_qq = ?")
        .bind(user_qq)
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn list_request_logs_by_user(
    pool: &SqlitePool,
    user_qq: &str,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<RequestLog>> {
    let logs = sqlx::query_as::<_, RequestLog>(
        "SELECT id, method, path, user_qq, body, status, timestamp
         FROM requestlog
         WHERE user_qq = ?
         ORDER BY timestamp DESC
         LIMIT ? OFFSET ?",
    )
    .bind(user_qq)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(logs)
}
