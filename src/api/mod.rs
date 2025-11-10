use crate::models::UserLpSummary;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use web_sys::window;

const API_BASE_URL: &str = "http://127.0.0.1:3000/api";
const TOKEN_KEY: &str = "jwt_token";

/// 从localStorage获取JWT token
pub fn get_token() -> Option<String> {
    let window = window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(TOKEN_KEY).ok()?
}

/// 保存JWT token到localStorage
pub fn set_token(token: &str) -> Result<(), String> {
    let window = window().ok_or("无法获取window对象")?;
    let storage = window
        .local_storage()
        .map_err(|_| "无法访问localStorage")?
        .ok_or("localStorage不可用")?;
    storage
        .set_item(TOKEN_KEY, token)
        .map_err(|_| "无法保存token")?;
    Ok(())
}

/// 清除JWT token
pub fn clear_token() -> Result<(), String> {
    let window = window().ok_or("无法获取window对象")?;
    let storage = window
        .local_storage()
        .map_err(|_| "无法访问localStorage")?
        .ok_or("localStorage不可用")?;
    storage
        .remove_item(TOKEN_KEY)
        .map_err(|_| "无法删除token")?;
    Ok(())
}

/// 错误响应
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}

// ============ 认证相关 ============

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub qq: String,
    pub nickname: String,
    pub password: String,
    pub birthday: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub qq: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    pub qq: String,
    pub nickname: String,
    pub birthday: Option<String>,
    pub role_name: Option<String>,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub is_default_password: bool, // 是否使用默认密码
}

#[derive(Debug, Deserialize)]
pub struct ProfileResponse {
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Default)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
    pub birthday: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 注册用户
pub async fn register(req: RegisterRequest) -> Result<RegisterResponse, String> {
    let response = Request::post(&format!("{}/auth/register", API_BASE_URL))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "注册失败".to_string(),
        });
        Err(error.message)
    }
}

/// 登录
pub async fn login(req: LoginRequest) -> Result<LoginResponse, String> {
    let response = Request::post(&format!("{}/auth/login", API_BASE_URL))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let login_resp: LoginResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        set_token(&login_resp.token)?;

        Ok(login_resp)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "登录失败".to_string(),
        });
        Err(error.message)
    }
}

/// 获取当前用户信息
pub async fn get_profile() -> Result<UserInfo, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/profile", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let profile: ProfileResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(profile.user)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取个人信息失败".to_string(),
        });
        Err(error.message)
    }
}

/// 更新个人信息
pub async fn update_profile(req: UpdateProfileRequest) -> Result<UserInfo, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::patch(&format!("{}/profile", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let profile: ProfileResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(profile.user)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "更新个人信息失败".to_string(),
        });
        Err(error.message)
    }
}

/// 修改密码
pub async fn change_password(old_password: String, new_password: String) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let payload = ChangePasswordRequest {
        old_password,
        new_password,
    };

    let response = Request::post(&format!("{}/profile/password", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&payload)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "修改密码失败".to_string(),
        });
        Err(error.message)
    }
}

// ============ 用户管理 ============

#[derive(Debug, Deserialize)]
pub struct UsersResponse {
    pub users: Vec<User>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub qq: String,
    pub nickname: String,
    pub password: String,
    pub birthday: Option<String>,
    pub main_role_id: Option<i64>,
    pub role_name: Option<String>,
}

/// 获取用户列表
pub async fn get_users() -> Result<Vec<User>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/users", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let users_resp: UsersResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(users_resp.users)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取用户列表失败".to_string(),
        });
        Err(error.message)
    }
}

/// 删除用户
pub async fn delete_user(qq: &str) -> Result<(), String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::delete(&format!("{}/users/{}", API_BASE_URL, qq))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        Ok(())
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "删除用户失败".to_string(),
        });
        Err(error.message)
    }
}

// ============ 权限管理 ============

#[derive(Debug, Deserialize)]
pub struct PermissionListResponse {
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionNamesResponse {
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Permission {
    pub name: String,
}

/// 获取所有权限
pub async fn get_permissions() -> Result<Vec<Permission>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/permissions", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let perms_resp: PermissionListResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(perms_resp.permissions)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取权限列表失败".to_string(),
        });
        Err(error.message)
    }
}

// ============ 角色管理 ============

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Role {
    pub role_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RolesResponse {
    pub roles: Vec<Role>,
}

#[derive(Debug, Serialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssignRoleRequest {
    pub user_qq: String,
    pub role_id: i64,
}

#[derive(Debug, Serialize)]
pub struct RolePermissionRequest {
    pub role_id: i64,
    pub permission_name: String,
}

#[derive(Debug, Deserialize)]
pub struct RolePermissionsResponse {
    pub permissions: Vec<String>,
}

/// 获取所有角色
pub async fn get_roles() -> Result<Vec<Role>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/roles", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let roles_resp: RolesResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(roles_resp.roles)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取角色列表失败".to_string(),
        });
        Err(error.message)
    }
}

/// 创建角色
pub async fn create_role(name: String, description: Option<String>) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let req = CreateRoleRequest { name, description };

    let response = Request::post(&format!("{}/roles/create", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "创建角色失败".to_string(),
        });
        Err(error.message)
    }
}

/// 删除角色
pub async fn delete_role(role_id: i64) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::delete(&format!("{}/roles/{}", API_BASE_URL, role_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "删除角色失败".to_string(),
        });
        Err(error.message)
    }
}

/// 给角色分配权限
pub async fn grant_permission_to_role(
    role_id: i64,
    permission_name: String,
) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let req = RolePermissionRequest {
        role_id,
        permission_name,
    };

    let response = Request::post(&format!("{}/roles/grant-permission", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "分配权限失败".to_string(),
        });
        Err(error.message)
    }
}

/// 从角色移除权限
pub async fn revoke_permission_from_role(
    role_id: i64,
    permission_name: String,
) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let req = RolePermissionRequest {
        role_id,
        permission_name,
    };

    let response = Request::post(&format!("{}/roles/revoke-permission", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "移除权限失败".to_string(),
        });
        Err(error.message)
    }
}

/// 获取角色的所有权限
pub async fn get_role_permissions(role_id: i64) -> Result<Vec<String>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/roles/{}/permissions", API_BASE_URL, role_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let perms_resp: RolePermissionsResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(perms_resp.permissions)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取角色权限失败".to_string(),
        });
        Err(error.message)
    }
}

/// 给用户分配角色
pub async fn assign_role_to_user(user_qq: String, role_id: i64) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let req = AssignRoleRequest { user_qq, role_id };

    let response = Request::post(&format!("{}/roles/assign", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "分配角色失败".to_string(),
        });
        Err(error.message)
    }
}

// ============ LP管理 ============

#[derive(Debug, Deserialize)]
pub struct LpTypesResponse {
    pub types: Vec<LpType>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LpType {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitLpRequest {
    pub upload_user_qq: String,
    pub user_qq: String,
    pub lp_type: i64,
    pub num: i32,
    pub reason: String,
    pub picture: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitLpResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct LpLogsResponse {
    pub logs: Vec<LpLog>,
}

#[derive(Debug, Deserialize)]
pub struct LpUserDetailResponse {
    pub summary: Option<UserLpSummary>,
    pub history: Vec<LpLog>,
}

#[derive(Debug, Deserialize)]
pub struct LpSummariesResponse {
    pub summaries: Vec<UserLpSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LpLog {
    pub id: Option<i64>,
    pub upload_time: String,
    pub upload_user_qq: String,
    pub user_qq: String,
    pub process_user_qq: Option<String>,
    pub role: Option<String>,
    pub lp_type: i64,
    pub num: i32,
    pub reason: String,
    pub status: i32,
    pub picture: Option<String>,
    pub process_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProcessLpRequest {
    pub id: i64,
    pub status: i32,
}

/// 获取LP类型
pub async fn get_lp_types() -> Result<Vec<LpType>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/lp/types", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let types_resp: LpTypesResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(types_resp.types)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取LP类型失败".to_string(),
        });
        Err(error.message)
    }
}

/// 提交LP
#[allow(clippy::too_many_arguments)]
pub async fn submit_lp(
    upload_user_qq: String,
    user_qq: String,
    lp_type: i64,
    num: i32,
    reason: String,
    picture: Option<String>,
    role: Option<String>,
) -> Result<SubmitLpResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let req = SubmitLpRequest {
        upload_user_qq,
        user_qq,
        lp_type,
        num,
        reason,
        picture,
        role,
    };

    let response = Request::post(&format!("{}/lp/submit", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "提交LP失败".to_string(),
        });
        Err(error.message)
    }
}

/// 获取LP日志
pub async fn get_lp_logs() -> Result<Vec<LpLog>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/lp/logs", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let logs_resp: LpLogsResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(logs_resp.logs)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取LP日志失败".to_string(),
        });
        Err(error.message)
    }
}

/// 处理LP审批
pub async fn process_lp(id: i64, status: i32) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let req = ProcessLpRequest { id, status };

    let response = Request::post(&format!("{}/lp/process", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "处理LP失败".to_string(),
        });
        Err(error.message)
    }
}

#[derive(Serialize)]
struct BatchProcessLpRequest {
    ids: Vec<i64>,
    status: i32,
}

#[derive(Deserialize)]
pub struct BatchProcessLpResponse {
    pub message: String,
    pub approved_count: u64,
    pub requested_count: usize,
}

/// 批量处理LP审批
pub async fn batch_process_lp(
    ids: Vec<i64>,
    status: i32,
) -> Result<BatchProcessLpResponse, String> {
    let token = get_token().ok_or("未登录")?;

    if ids.is_empty() {
        return Err("请至少选择一条申请".to_string());
    }

    let req = BatchProcessLpRequest { ids, status };

    let response = Request::post(&format!("{}/lp/batch-process", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&req)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "批量处理LP失败".to_string(),
        });
        Err(error.message)
    }
}

/// 查询指定用户的LP详情
pub async fn get_user_lp_detail(user_qq: &str) -> Result<LpUserDetailResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/lp/user/{}", API_BASE_URL, user_qq))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "查询用户LP失败".to_string(),
        });
        Err(error.message)
    }
}

/// 获取所有用户LP汇总
pub async fn get_lp_summaries() -> Result<Vec<UserLpSummary>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/lp/summaries", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let resp: LpSummariesResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(resp.summaries)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取LP汇总失败".to_string(),
        });
        Err(error.message)
    }
}

// ============ 抽奖管理 ============

#[derive(Debug, Deserialize)]
pub struct LuckyDrawsResponse {
    pub draws: Vec<LuckyDraw>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LuckyDraw {
    pub id: Option<i64>,
    pub create_time: String,
    pub create_qq: String,
    pub item_id: Option<i64>,
    pub fitting: Option<String>,
    pub num: i32,
    pub min_lp_require: i32,
    pub plan_time: String,
    pub status: i32,
    pub winner_qq: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateDrawPayload {
    pub create_qq: String,
    pub item_id: Option<i64>,
    pub fitting: Option<String>,
    pub num: i32,
    pub min_lp_require: i32,
    pub plan_time: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDrawResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteDrawResponse {
    pub message: String,
    #[serde(default)]
    pub winner: Option<String>,
}

/// 获取抽奖列表
pub async fn get_lucky_draws() -> Result<Vec<LuckyDraw>, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!("{}/lucky-draw", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let draws_resp: LuckyDrawsResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(draws_resp.draws)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取抽奖列表失败".to_string(),
        });
        Err(error.message)
    }
}

/// 创建抽奖
pub async fn create_lucky_draw(payload: CreateDrawPayload) -> Result<CreateDrawResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::post(&format!("{}/lucky-draw/create", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&payload)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "创建抽奖失败".to_string(),
        });
        Err(error.message)
    }
}

/// 执行抽奖
pub async fn execute_lucky_draw(draw_id: i64) -> Result<ExecuteDrawResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::post(&format!("{}/lucky-draw/execute/{}", API_BASE_URL, draw_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "执行抽奖失败".to_string(),
        });
        Err(error.message)
    }
}

/// 删除抽奖
pub async fn delete_lucky_draw(draw_id: i64) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::delete(&format!("{}/lucky-draw/{}", API_BASE_URL, draw_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "删除抽奖失败".to_string(),
        });
        Err(error.message)
    }
}

// ============ 商店管理 ============

#[derive(Debug, Deserialize)]
pub struct ShopItemsResponse {
    pub items: Vec<ShopItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShopItem {
    pub id: Option<i64>,
    pub count: i32,
    pub price: String,
    pub name: String,
    pub seller: String,
    pub location: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Debug, Serialize)]
pub struct CreateItemPayload {
    pub count: i32,
    pub price: String,
    pub name: String,
    pub seller: String,
    pub location: String,
}

#[derive(Debug, Serialize)]
pub struct PurchasePayload {
    pub buyer: String,
    pub item_id: i64,
    pub count: i32,
}

/// 获取商店物品列表
pub async fn get_shop_items(seller_qq: Option<&str>) -> Result<Vec<ShopItem>, String> {
    let token = get_token().ok_or("未登录")?;

    let url = if let Some(qq) = seller_qq {
        format!("{}/shop/items/my?seller={}", API_BASE_URL, qq)
    } else {
        format!("{}/shop/items", API_BASE_URL)
    };

    let response = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let items_resp: ShopItemsResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(items_resp.items)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取商店物品失败".to_string(),
        });
        Err(error.message)
    }
}

/// 创建商店物品
pub async fn create_shop_item(payload: CreateItemPayload) -> Result<CreateItemResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::post(&format!("{}/shop/items/create", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&payload)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "创建商店物品失败".to_string(),
        });
        Err(error.message)
    }
}

/// 购买商店物品
pub async fn purchase_shop_item(buyer: String, item_id: i64, count: i32) -> Result<String, String> {
    let token = get_token().ok_or("未登录")?;

    let payload = PurchasePayload {
        buyer,
        item_id,
        count,
    };

    let response = Request::post(&format!("{}/shop/purchase", API_BASE_URL))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&payload)
        .map_err(|e| format!("序列化请求失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        let msg_resp: MessageResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(msg_resp.message)
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "购买失败".to_string(),
        });
        Err(error.message)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShopLog {
    pub id: Option<i64>,
    pub buyer: String,
    pub count: i32,
    pub price: String,
    pub name: String,
    pub time: String,
    pub seller: String,
    pub location: String,
}

#[derive(Debug, Deserialize)]
pub struct UserTransactionsResponse {
    pub purchases: Vec<ShopLog>,
    pub sales: Vec<ShopLog>,
}

/// 获取用户的交易记录（购买和销售）
pub async fn get_shop_transactions(user_qq: &str) -> Result<UserTransactionsResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let response = Request::get(&format!(
        "{}/shop/transactions?user_qq={}",
        API_BASE_URL, user_qq
    ))
    .header("Authorization", &format!("Bearer {}", token))
    .send()
    .await
    .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取交易记录失败".to_string(),
        });
        Err(error.message)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestLog {
    pub id: i64,
    pub method: String,
    pub path: String,
    pub user_qq: Option<String>,
    pub body: Option<String>,
    pub status: i32,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct LogListResponse {
    pub logs: Vec<RequestLog>,
    pub total: i64,
}

/// 获取请求日志
pub async fn get_request_logs(
    limit: Option<i32>,
    offset: Option<i32>,
    user_qq: Option<String>,
) -> Result<LogListResponse, String> {
    let token = get_token().ok_or("未登录")?;

    let mut url = format!("{}/logs", API_BASE_URL);
    let mut params = Vec::new();

    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }
    if let Some(qq) = user_qq {
        if !qq.is_empty() {
            params.push(format!("user_qq={}", qq));
        }
    }

    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let response = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.ok() {
        response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))
    } else {
        let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
            message: "获取日志失败".to_string(),
        });
        Err(error.message)
    }
}
