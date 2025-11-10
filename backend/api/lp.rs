use crate::auth::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use team_operation_system::db::{record_request_log, LpService};

#[derive(Deserialize)]
pub struct SubmitLpRequest {
    pub upload_user_qq: String,
    pub user_qq: String,
    pub lp_type: i64,
    pub num: i32,
    pub reason: String,
    pub picture: Option<String>,
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct ProcessLpRequest {
    pub id: i64,
    pub status: i32,
}

pub async fn list_lp_types(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let types = match LpService::get_all_lp_types(&state.pool).await {
        Ok(types) => types,
        Err(e) => {
            log::error!("获取LP类型失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/lp/types",
                Some(auth_user.qq()),
                Some(format!("获取LP类型失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/lp/types",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "types": types })))
}

pub async fn submit_lp(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<SubmitLpRequest>,
) -> Result<Json<Value>, StatusCode> {
    let SubmitLpRequest {
        upload_user_qq,
        user_qq,
        lp_type,
        num,
        reason,
        picture,
        role,
    } = payload;

    if upload_user_qq != auth_user.qq() && !auth_user.has_permission("审核LP") {
        return Err(StatusCode::FORBIDDEN);
    }

    let id = match LpService::submit_lp_request(
        &state.pool,
        &upload_user_qq,
        &user_qq,
        lp_type,
        num,
        &reason,
        picture.clone(),
        role.clone(),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("提交LP失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/lp/submit",
                Some(auth_user.qq()),
                Some(format!("提交LP失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "POST",
        "/lp/submit",
        Some(auth_user.qq()),
        Some(
            serde_json::to_string(&json!({
                "upload_user_qq": upload_user_qq,
                "user_qq": user_qq,
                "lp_type": lp_type,
                "num": num,
                "reason": reason,
                "role": role,
            }))
            .unwrap_or_default(),
        ),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "message": "LP申请提交成功", "id": id })))
}

pub async fn list_lp_logs(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    if !auth_user.has_permission("审核LP") && !auth_user.has_permission("查看日志") {
        return Err(StatusCode::FORBIDDEN);
    }

    let logs = match LpService::get_all_lp_logs(&state.pool).await {
        Ok(logs) => logs,
        Err(e) => {
            log::error!("获取LP日志失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/lp/logs",
                Some(auth_user.qq()),
                Some(format!("获取LP日志失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/lp/logs",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "logs": logs })))
}

pub async fn process_lp(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<ProcessLpRequest>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("审核LP")?;

    if let Err(e) =
        LpService::process_lp_request(&state.pool, payload.id, auth_user.qq(), payload.status).await
    {
        log::error!("处理LP失败: {}", e);
        let _ = record_request_log(
            &state.pool,
            "POST",
            "/lp/process",
            Some(auth_user.qq()),
            Some(format!("处理LP失败: {}", e)),
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let _ = record_request_log(
        &state.pool,
        "POST",
        "/lp/process",
        Some(auth_user.qq()),
        Some(
            serde_json::to_string(&json!({
                "id": payload.id,
                "status": payload.status,
            }))
            .unwrap_or_default(),
        ),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "message": "LP审批完成" })))
}

#[derive(Deserialize)]
pub struct BatchProcessLpRequest {
    pub ids: Vec<i64>,
    pub status: i32,
}

/// 批量审批LP申请
pub async fn batch_process_lp(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<BatchProcessLpRequest>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("审核LP")?;

    if payload.ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let approved_count = match team_operation_system::db::batch_approve_lp(
        &state.pool,
        &payload.ids,
        auth_user.qq(),
        payload.status,
    )
    .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("批量处理LP失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/lp/batch-process",
                Some(auth_user.qq()),
                Some(format!("批量处理LP失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "POST",
        "/lp/batch-process",
        Some(auth_user.qq()),
        Some(
            serde_json::to_string(&json!({
                "ids": payload.ids,
                "status": payload.status,
                "approved_count": approved_count,
            }))
            .unwrap_or_default(),
        ),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({
        "message": "批量审批完成",
        "approved_count": approved_count,
        "requested_count": payload.ids.len(),
    })))
}

pub async fn user_lp_detail(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(user_qq): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    if auth_user.qq() != user_qq
        && !auth_user.has_permission("审核LP")
        && !auth_user.has_permission("查看日志")
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let summary = match LpService::get_user_lp_summary(&state.pool, &user_qq).await {
        Ok(summary) => summary,
        Err(e) => {
            log::error!("获取用户LP汇总失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                &format!("/lp/user/{}", user_qq),
                Some(auth_user.qq()),
                Some(format!("获取LP汇总失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let history = match LpService::get_user_lp_history(&state.pool, &user_qq).await {
        Ok(history) => history,
        Err(e) => {
            log::error!("获取用户LP历史失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                &format!("/lp/user/{}", user_qq),
                Some(auth_user.qq()),
                Some(format!("获取LP历史失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        &format!("/lp/user/{}", user_qq),
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({
        "summary": summary,
        "history": history,
    })))
}

pub async fn list_lp_summaries(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    if !auth_user.has_permission("审核LP") && !auth_user.has_permission("查看日志") {
        return Err(StatusCode::FORBIDDEN);
    }

    let summaries = match LpService::get_all_lp_summaries(&state.pool).await {
        Ok(summaries) => summaries,
        Err(e) => {
            log::error!("获取LP汇总列表失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/lp/summaries",
                Some(auth_user.qq()),
                Some(format!("获取LP汇总列表失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/lp/summaries",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "summaries": summaries })))
}
