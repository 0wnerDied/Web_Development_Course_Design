use crate::auth::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use team_operation_system::db::{record_request_log, RoleService};

/// 检查指定用户是否是第一个默认管理员（数据库中第一个创建的用户）
async fn is_first_admin(pool: &sqlx::SqlitePool, qq: &str) -> bool {
    let first_user_qq: Option<String> =
        sqlx::query_scalar("SELECT qq FROM user ORDER BY rowid LIMIT 1")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    Some(qq) == first_user_qq.as_deref()
}

#[derive(Deserialize, serde::Serialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, serde::Serialize)]
pub struct AssignRoleRequest {
    pub user_qq: String,
    pub role_id: i64,
}

#[derive(Deserialize, serde::Serialize)]
pub struct RolePermissionRequest {
    pub role_id: i64,
    pub permission_name: String,
}

/// 获取所有角色
pub async fn list_roles(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    let roles = match RoleService::get_all_roles(&state.pool).await {
        Ok(roles) => roles,
        Err(e) => {
            log::error!("获取角色列表失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/roles",
                Some(auth_user.qq()),
                Some(format!("获取角色列表失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/roles",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "roles": roles })))
}

/// 创建角色
pub async fn create_role(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateRoleRequest>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    let role_id =
        match RoleService::create_role(&state.pool, &payload.name, payload.description.as_deref())
            .await
        {
            Ok(id) => id,
            Err(e) => {
                log::error!("创建角色失败: {}", e);
                let _ = record_request_log(
                    &state.pool,
                    "POST",
                    "/roles/create",
                    Some(auth_user.qq()),
                    Some(format!("创建角色失败: {}", e)),
                    StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
                )
                .await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    log::info!("角色创建: ID={}, 名称={}", role_id, payload.name);
    let _ = record_request_log(
        &state.pool,
        "POST",
        "/roles/create",
        Some(auth_user.qq()),
        Some(
            serde_json::to_string(&json!({
                "name": payload.name,
                "description": payload.description,
            }))
            .unwrap_or_default(),
        ),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(
        json!({ "message": "角色创建成功", "role_id": role_id }),
    ))
}

/// 删除角色
pub async fn delete_role(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(role_id): Path<i64>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    match RoleService::delete_role(&state.pool, role_id).await {
        Ok(_) => {
            log::info!("角色删除: ID={}", role_id);
            let _ = record_request_log(
                &state.pool,
                "DELETE",
                &format!("/roles/{}", role_id),
                Some(auth_user.qq()),
                None,
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "角色删除成功" })))
        }
        Err(e) => {
            log::error!("删除角色失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "DELETE",
                &format!("/roles/{}", role_id),
                Some(auth_user.qq()),
                Some(format!("删除角色失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 给角色分配权限
pub async fn grant_permission_to_role(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<RolePermissionRequest>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    match RoleService::grant_permission_to_role(
        &state.pool,
        payload.role_id,
        &payload.permission_name,
    )
    .await
    {
        Ok(_) => {
            log::info!(
                "角色权限分配: role_id={}, permission={}",
                payload.role_id,
                payload.permission_name
            );
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/roles/grant-permission",
                Some(auth_user.qq()),
                Some(serde_json::to_string(&payload).unwrap_or_default()),
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "权限分配成功" })))
        }
        Err(e) => {
            log::error!("角色权限分配失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/roles/grant-permission",
                Some(auth_user.qq()),
                Some(format!("权限分配失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 从角色移除权限
pub async fn revoke_permission_from_role(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<RolePermissionRequest>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    match RoleService::revoke_permission_from_role(
        &state.pool,
        payload.role_id,
        &payload.permission_name,
    )
    .await
    {
        Ok(_) => {
            log::info!(
                "角色权限移除: role_id={}, permission={}",
                payload.role_id,
                payload.permission_name
            );
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/roles/revoke-permission",
                Some(auth_user.qq()),
                Some(serde_json::to_string(&payload).unwrap_or_default()),
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "权限移除成功" })))
        }
        Err(e) => {
            log::error!("角色权限移除失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/roles/revoke-permission",
                Some(auth_user.qq()),
                Some(format!("权限移除失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取角色的所有权限
pub async fn get_role_permissions(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(role_id): Path<i64>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    let permissions = match RoleService::get_role_permissions(&state.pool, role_id).await {
        Ok(perms) => perms,
        Err(e) => {
            log::error!("获取角色权限失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                &format!("/roles/{}/permissions", role_id),
                Some(auth_user.qq()),
                Some(format!("获取角色权限失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        &format!("/roles/{}/permissions", role_id),
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "permissions": permissions })))
}

/// 给用户分配角色
pub async fn assign_role_to_user(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<AssignRoleRequest>,
) -> Result<Json<Value>, StatusCode> {
    // 需要"管理角色"权限
    if !auth_user.has_permission("管理角色") {
        return Err(StatusCode::FORBIDDEN);
    }

    // 保护第一个默认管理员账号，不允许修改其角色
    if is_first_admin(&state.pool, &payload.user_qq).await {
        log::warn!("尝试修改第一个默认管理员的角色: {}", payload.user_qq);
        let _ = record_request_log(
            &state.pool,
            "POST",
            "/roles/assign",
            Some(auth_user.qq()),
            Some("拒绝操作：不能修改第一个默认管理员的角色".to_string()),
            StatusCode::FORBIDDEN.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::FORBIDDEN);
    }

    match RoleService::assign_main_role(&state.pool, &payload.user_qq, payload.role_id).await {
        Ok(_) => {
            log::info!(
                "用户角色分配: user={}, role_id={}",
                payload.user_qq,
                payload.role_id
            );
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/roles/assign",
                Some(auth_user.qq()),
                Some(serde_json::to_string(&payload).unwrap_or_default()),
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "角色分配成功" })))
        }
        Err(e) => {
            log::error!("用户角色分配失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/roles/assign",
                Some(auth_user.qq()),
                Some(format!("角色分配失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
