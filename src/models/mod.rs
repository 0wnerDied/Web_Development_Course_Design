use serde::{Deserialize, Serialize};

#[cfg(feature = "backend")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct User {
    pub qq: String,
    pub main_role_id: Option<i64>,
    pub nickname: String,
    pub password: String,
    pub birthday: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct UserWithRole {
    pub qq: String,
    pub main_role_id: Option<i64>,
    pub nickname: String,
    pub password: String,
    pub birthday: Option<String>,
    pub role_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct Role {
    pub role_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub qq: String,
    pub nickname: String,
    pub birthday: Option<String>,
    pub main_role_id: Option<i64>,
    pub role_name: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct Permission {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct PermissionLog {
    pub id: i64,
    pub permission_name: String,
    pub user_qq: String,
    pub operator_qq: String,
    pub action: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct LpType {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
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
    pub status: i32, // 0: 待处理, 1: 已通过, 2: 已拒绝
    pub picture: Option<String>,
    pub process_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct LuckyDrawLog {
    pub id: Option<i64>,
    pub create_time: String,
    pub create_qq: String,
    pub item_id: Option<i64>,
    pub fitting: Option<String>,
    pub num: i32,
    pub min_lp_require: i32,
    pub plan_time: String,
    pub status: i32, // 0: 未开奖, 1: 已开奖
    pub winner_qq: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct ShopItem {
    pub id: Option<i64>,
    pub count: i32,
    pub price: String,
    pub name: String,
    pub seller: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct UserLpSummary {
    pub qq: String,
    pub nickname: String,
    pub total_lp: i64,
    pub pending_count: i64,
    pub approved_count: i64,
    pub rejected_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "backend", derive(FromRow))]
pub struct RequestLog {
    pub id: i64,
    pub method: String,
    pub path: String,
    pub user_qq: Option<String>,
    pub body: Option<String>,
    pub status: i32,
    pub timestamp: String,
}
