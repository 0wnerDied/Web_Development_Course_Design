use crate::{auth::AuthenticatedUser, state::AppState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use team_operation_system::{db, models::RequestLog};

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    limit: Option<i32>,
    offset: Option<i32>,
    user_qq: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogListResponse {
    logs: Vec<RequestLog>,
    total: i64,
}

/// 列出请求日志
pub async fn list_logs(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> impl IntoResponse {
    // 检查权限
    if let Err(status) = auth_user.require_permission("查看日志") {
        let message = match status {
            StatusCode::FORBIDDEN => "无权限查看日志",
            StatusCode::INTERNAL_SERVER_ERROR => "权限校验失败",
            _ => "身份验证失败",
        };
        return (status, Json(serde_json::json!({"message": message}))).into_response();
    }

    let LogQuery {
        limit,
        offset,
        user_qq,
    } = query;

    let limit_value = limit.unwrap_or(100).clamp(1, 500) as i64;
    let offset_value = offset.unwrap_or(0).max(0) as i64;
    let user_filter = user_qq
        .as_deref()
        .map(|qq| qq.trim())
        .filter(|qq| !qq.is_empty())
        .map(|qq| qq.to_string());

    // 如果指定了用户QQ，则查询该用户的日志
    let logs = if let Some(user_qq) = user_filter.as_ref() {
        db::list_request_logs_by_user(&state.pool, user_qq, limit_value, offset_value).await
    } else {
        db::list_request_logs(&state.pool, limit_value, offset_value).await
    };

    let logs = match logs {
        Ok(l) => l,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"message": format!("查询日志失败: {}", e)})),
            )
                .into_response()
        }
    };

    let total = match user_filter.as_ref() {
        Some(user_qq) => match db::count_request_logs_by_user(&state.pool, user_qq).await {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"message": format!("统计日志失败: {}", e)})),
                )
                    .into_response()
            }
        },
        None => match db::count_request_logs(&state.pool).await {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"message": format!("统计日志失败: {}", e)})),
                )
                    .into_response()
            }
        },
    };

    (StatusCode::OK, Json(LogListResponse { logs, total })).into_response()
}
