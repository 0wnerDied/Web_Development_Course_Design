mod log;
mod lp;
mod lucky_draw;
mod permission;
mod role;
mod shop;
mod user;

use crate::state::AppState;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        // 用户相关
        .route("/auth/register", post(user::register))
        .route("/auth/login", post(user::login))
        .route("/profile", get(user::profile).patch(user::update_profile))
        .route("/profile/password", post(user::change_password))
        .route("/users", get(user::list_users))
        .route(
            "/users/{qq}",
            patch(user::update_user).delete(user::delete_user),
        )
        // 权限相关（仅用于角色管理中获取权限列表）
        .route("/permissions", get(permission::list_permissions))
        // 角色相关
        .route("/roles", get(role::list_roles))
        .route("/roles/create", post(role::create_role))
        .route("/roles/{role_id}", delete(role::delete_role))
        .route(
            "/roles/grant-permission",
            post(role::grant_permission_to_role),
        )
        .route(
            "/roles/revoke-permission",
            post(role::revoke_permission_from_role),
        )
        .route(
            "/roles/{role_id}/permissions",
            get(role::get_role_permissions),
        )
        .route("/roles/assign", post(role::assign_role_to_user))
        // LP 相关
        .route("/lp/types", get(lp::list_lp_types))
        .route("/lp/submit", post(lp::submit_lp))
        .route("/lp/logs", get(lp::list_lp_logs))
        .route("/lp/process", post(lp::process_lp))
        .route("/lp/batch-process", post(lp::batch_process_lp))
        .route("/lp/user/{qq}", get(lp::user_lp_detail))
        .route("/lp/summaries", get(lp::list_lp_summaries))
        // 抽奖相关
        .route("/lucky-draw", get(lucky_draw::list_draws))
        .route("/lucky-draw/create", post(lucky_draw::create_draw))
        .route("/lucky-draw/execute/{id}", post(lucky_draw::execute_draw))
        .route("/lucky-draw/{id}", delete(lucky_draw::delete_draw))
        .route(
            "/lucky-draw/winner/{id}",
            post(lucky_draw::set_manual_winner),
        )
        // 商店相关
        .route("/shop/items", get(shop::list_items))
        .route("/shop/items/my", get(shop::my_items))
        .route("/shop/items/create", post(shop::create_item))
        .route("/shop/purchase", post(shop::purchase_item))
        .route("/shop/transactions", get(shop::get_user_transactions))
        // 日志相关
        .route("/logs", get(log::list_logs))
}
