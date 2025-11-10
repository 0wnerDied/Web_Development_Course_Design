use crate::auth::AuthenticatedUser;
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use team_operation_system::db::{record_request_log, PermissionService};

pub async fn list_permissions(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("管理角色")?;
    let permissions = match PermissionService::get_all_permissions(&state.pool).await {
        Ok(perms) => perms,
        Err(e) => {
            log::error!("获取权限列表失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/permissions",
                Some(auth_user.qq()),
                Some(format!("获取权限失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/permissions",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "permissions": permissions })))
}
