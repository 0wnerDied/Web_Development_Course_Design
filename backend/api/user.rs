use crate::auth::{
    AuthenticatedUser, Claims, LoginRequest, LoginResponse, RegisterRequest, UserInfo, JWT_SECRET,
};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use team_operation_system::db::{record_request_log, PermissionService, RoleService, UserService};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub user: UserInfo,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<Value>, StatusCode> {
    let RegisterRequest {
        qq,
        nickname,
        password,
        birthday,
    } = payload;

    let log_body = json!({
        "qq": qq,
        "nickname": nickname,
        "birthday": birthday,
    });

    match UserService::register(
        &state.pool,
        log_body["qq"].as_str().unwrap(),
        log_body["nickname"].as_str().unwrap(),
        &password,
        log_body["birthday"].as_str(),
    )
    .await
    {
        Ok(_) => {
            let log_body_str = serde_json::to_string(&log_body).unwrap_or_default();
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/auth/register",
                log_body["qq"].as_str(),
                Some(log_body_str),
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "注册成功" })))
        }
        Err(e) => {
            log::error!("注册失败: {}", e);
            let error_body = format!("注册失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/auth/register",
                log_body["qq"].as_str(),
                Some(error_body),
                StatusCode::BAD_REQUEST.as_u16() as i32,
            )
            .await;
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user = match UserService::login(&state.pool, &payload.qq, &payload.password).await {
        Ok(user) => user,
        Err(e) => {
            log::error!("登录查询失败: {}", e);
            let error_body = format!("查询失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/auth/login",
                Some(&payload.qq),
                Some(error_body),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let user = match user {
        Some(u) => u,
        None => {
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/auth/login",
                Some(&payload.qq),
                Some("登录失败: 未授权".to_string()),
                StatusCode::UNAUTHORIZED.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let permissions = PermissionService::get_user_permissions(&state.pool, &user.qq)
        .await
        .unwrap_or_else(|e| {
            log::error!("查询用户权限失败: {}", e);
            Vec::new()
        });

    let role_name = RoleService::get_user_role(&state.pool, &user.qq)
        .await
        .ok()
        .flatten()
        .map(|r| r.name);

    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("有效时间")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.qq.clone(),
        nickname: user.nickname.clone(),
        exp,
        permissions: permissions.clone(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 检测是否使用默认密码
    // 方法：检查是否是数据库中第一个创建的用户（初始化时创建的默认管理员）
    let is_default_password = {
        // 查询数据库中第一个用户的QQ号
        let first_user_qq: Option<String> =
            sqlx::query_scalar("SELECT qq FROM user ORDER BY rowid LIMIT 1")
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();

        // 如果当前用户是第一个用户，检查是否使用默认密码 "admin@666"
        if Some(&user.qq) == first_user_qq.as_ref() {
            bcrypt::verify("admin@666", &user.password).unwrap_or(false)
        } else {
            false
        }
    };

    let user_info = UserInfo {
        qq: user.qq,
        nickname: user.nickname,
        birthday: user.birthday,
        role_name,
        permissions,
        is_default_password,
    };

    let login_body = json!({
        "qq": &user_info.qq,
    });
    let login_body_str = serde_json::to_string(&login_body).unwrap_or_default();
    let _ = record_request_log(
        &state.pool,
        "POST",
        "/auth/login",
        Some(&user_info.qq),
        Some(login_body_str),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(LoginResponse {
        token,
        user: user_info,
    }))
}

pub async fn list_users(
    user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    user.require_permission("用户管理")?;
    let users = match UserService::get_all_users(&state.pool).await {
        Ok(users) => users,
        Err(e) => {
            log::error!("获取用户列表失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/users",
                Some(user.qq()),
                Some(format!("获取用户失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/users",
        Some(user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "users": users })))
}

pub async fn delete_user(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(qq): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    user.require_permission("用户管理")?;

    // 保护第一个默认管理员账号，不允许删除
    if is_first_admin(&state.pool, &qq).await {
        log::warn!("尝试删除第一个默认管理员账号: {}", qq);
        let _ = record_request_log(
            &state.pool,
            "DELETE",
            &format!("/users/{}", qq),
            Some(user.qq()),
            Some("拒绝删除：不能删除第一个默认管理员账号".to_string()),
            StatusCode::FORBIDDEN.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::FORBIDDEN);
    }

    if let Err(e) = UserService::delete_user(&state.pool, &qq).await {
        log::error!("删除用户失败: {}", e);
        let err_body = format!("删除用户失败: {}", e);
        let _ = record_request_log(
            &state.pool,
            "DELETE",
            &format!("/users/{}", qq),
            Some(user.qq()),
            Some(err_body),
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let _ = record_request_log(
        &state.pool,
        "DELETE",
        &format!("/users/{}", qq),
        Some(user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "message": "删除成功" })))
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub nickname: Option<String>,
    pub birthday: Option<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

pub async fn update_user(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(qq): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<Value>, StatusCode> {
    let mut requires_admin = false;
    if auth_user.qq() != qq {
        requires_admin = true;
    }
    if requires_admin {
        auth_user.require_permission("用户管理")?;
    }

    if let Err(e) = UserService::update_user(
        &state.pool,
        &qq,
        payload.nickname.clone(),
        payload.birthday.clone(),
    )
    .await
    {
        log::error!("更新用户信息失败: {}", e);
        let err_body = format!("更新用户信息失败: {}", e);
        let _ = record_request_log(
            &state.pool,
            "PATCH",
            &format!("/users/{}", qq),
            Some(auth_user.qq()),
            Some(err_body),
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let log_body = json!({
        "nickname": payload.nickname,
        "birthday": payload.birthday,
    });
    let _ = record_request_log(
        &state.pool,
        "PATCH",
        &format!("/users/{}", qq),
        Some(auth_user.qq()),
        Some(log_body.to_string()),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "message": "更新成功" })))
}

pub async fn profile(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<ProfileResponse>, StatusCode> {
    let user = match UserService::get_user(&state.pool, auth_user.qq()).await {
        Ok(user) => user,
        Err(e) => {
            log::error!("获取个人信息失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/profile",
                Some(auth_user.qq()),
                Some(format!("获取个人信息失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let user = match user {
        Some(user) => user,
        None => {
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/profile",
                Some(auth_user.qq()),
                Some("用户不存在".to_string()),
                StatusCode::NOT_FOUND.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let permissions = PermissionService::get_user_permissions(&state.pool, auth_user.qq())
        .await
        .unwrap_or_else(|e| {
            log::error!("获取权限失败: {}", e);
            Vec::new()
        });
    let role_name = RoleService::get_user_role(&state.pool, auth_user.qq())
        .await
        .ok()
        .flatten()
        .map(|r| r.name);

    let info = UserInfo {
        qq: user.qq,
        nickname: user.nickname,
        birthday: user.birthday,
        role_name,
        permissions,
        is_default_password: false, // 个人信息接口不返回此字段
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/profile",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(ProfileResponse { user: info }))
}

pub async fn update_profile(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<ProfileResponse>, StatusCode> {
    if let Err(e) = UserService::update_user(
        &state.pool,
        auth_user.qq(),
        payload.nickname.clone(),
        payload.birthday.clone(),
    )
    .await
    {
        log::error!("更新个人信息失败: {}", e);
        let _ = record_request_log(
            &state.pool,
            "PATCH",
            "/profile",
            Some(auth_user.qq()),
            Some(format!("更新个人信息失败: {}", e)),
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let updated_user = match UserService::get_user(&state.pool, auth_user.qq()).await {
        Ok(user) => user,
        Err(e) => {
            log::error!("更新后获取个人信息失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "PATCH",
                "/profile",
                Some(auth_user.qq()),
                Some(format!("更新后获取个人信息失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let updated_user = match updated_user {
        Some(user) => user,
        None => {
            let _ = record_request_log(
                &state.pool,
                "PATCH",
                "/profile",
                Some(auth_user.qq()),
                Some("用户不存在".to_string()),
                StatusCode::NOT_FOUND.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let permissions = PermissionService::get_user_permissions(&state.pool, auth_user.qq())
        .await
        .unwrap_or_else(|e| {
            log::error!("获取权限失败: {}", e);
            Vec::new()
        });
    let role_name = RoleService::get_user_role(&state.pool, auth_user.qq())
        .await
        .ok()
        .flatten()
        .map(|r| r.name);

    let info = UserInfo {
        qq: updated_user.qq,
        nickname: updated_user.nickname,
        birthday: updated_user.birthday,
        role_name,
        permissions,
        is_default_password: false, // 个人信息接口不返回此字段
    };

    let body = json!({
        "nickname": payload.nickname,
        "birthday": payload.birthday,
    });
    let _ = record_request_log(
        &state.pool,
        "PATCH",
        "/profile",
        Some(auth_user.qq()),
        Some(body.to_string()),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(ProfileResponse { user: info }))
}

pub async fn change_password(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<Value>, StatusCode> {
    if payload.new_password.len() < 6 {
        let _ = record_request_log(
            &state.pool,
            "POST",
            "/profile/password",
            Some(auth_user.qq()),
            Some("新密码长度不足".to_string()),
            StatusCode::BAD_REQUEST.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::BAD_REQUEST);
    }

    match UserService::change_password(
        &state.pool,
        auth_user.qq(),
        &payload.old_password,
        &payload.new_password,
    )
    .await
    {
        Ok(true) => {
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/profile/password",
                Some(auth_user.qq()),
                None,
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "密码修改成功" })))
        }
        Ok(false) => {
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/profile/password",
                Some(auth_user.qq()),
                Some("原密码错误".to_string()),
                StatusCode::UNAUTHORIZED.as_u16() as i32,
            )
            .await;
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            log::error!("修改密码失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/profile/password",
                Some(auth_user.qq()),
                Some(format!("修改密码失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
