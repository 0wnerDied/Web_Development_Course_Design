use crate::auth::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use team_operation_system::db::{record_request_log, ShopService};

#[derive(Deserialize)]
pub struct CreateItemRequest {
    pub count: i32,
    pub price: String,
    pub name: String,
    pub seller: String,
    pub location: String,
}

#[derive(Deserialize)]
pub struct PurchaseRequest {
    pub buyer: String,
    pub item_id: i64,
    pub count: i32,
}

#[derive(Deserialize)]
pub struct MyItemsQuery {
    pub seller: String,
}

pub async fn list_items(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let items = match ShopService::get_all_items(&state.pool).await {
        Ok(items) => items,
        Err(e) => {
            log::error!("获取商品列表失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/shop/items",
                Some(auth_user.qq()),
                Some(format!("获取商品失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/shop/items",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "items": items })))
}

pub async fn my_items(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<MyItemsQuery>,
) -> Result<Json<Value>, StatusCode> {
    if auth_user.qq() != params.seller && !auth_user.has_permission("管理商品") {
        return Err(StatusCode::FORBIDDEN);
    }

    let items = match ShopService::get_user_items(&state.pool, &params.seller).await {
        Ok(items) => items,
        Err(e) => {
            log::error!("获取用户商品失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                &format!("/shop/items/my?seller={}", params.seller),
                Some(auth_user.qq()),
                Some(format!("获取用户商品失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        &format!("/shop/items/my?seller={}", params.seller),
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "items": items })))
}

pub async fn create_item(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateItemRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth_user.qq() != payload.seller && !auth_user.has_permission("管理商品") {
        return Err(StatusCode::FORBIDDEN);
    }

    let CreateItemRequest {
        count,
        price,
        name,
        seller,
        location,
    } = payload;

    let id =
        match ShopService::add_item(&state.pool, count, &price, &name, &seller, &location).await {
            Ok(id) => id,
            Err(e) => {
                log::error!("商品上架失败: {}", e);
                let _ = record_request_log(
                    &state.pool,
                    "POST",
                    "/shop/items/create",
                    Some(auth_user.qq()),
                    Some(format!("商品上架失败: {}", e)),
                    StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
                )
                .await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    log::info!("商品上架: ID={}, 卖家={}", id, seller);
    let _ = record_request_log(
        &state.pool,
        "POST",
        "/shop/items/create",
        Some(auth_user.qq()),
        Some(
            serde_json::to_string(&json!({
                "name": name,
                "count": count,
                "price": price,
                "location": location,
            }))
            .unwrap_or_default(),
        ),
        StatusCode::OK.as_u16() as i32,
    )
    .await;
    Ok(Json(json!({ "message": "商品上架成功", "id": id })))
}

pub async fn purchase_item(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<PurchaseRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth_user.qq() != payload.buyer {
        return Err(StatusCode::FORBIDDEN);
    }

    let success = match ShopService::purchase_item(
        &state.pool,
        &payload.buyer,
        payload.item_id,
        payload.count,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            log::error!("购买失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/shop/purchase",
                Some(auth_user.qq()),
                Some(format!("购买失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if success {
        log::info!(
            "购买成功: 买家={}, 商品ID={}",
            payload.buyer,
            payload.item_id
        );
        let _ = record_request_log(
            &state.pool,
            "POST",
            "/shop/purchase",
            Some(auth_user.qq()),
            Some(
                serde_json::to_string(&json!({
                    "item_id": payload.item_id,
                    "count": payload.count,
                }))
                .unwrap_or_default(),
            ),
            StatusCode::OK.as_u16() as i32,
        )
        .await;
        Ok(Json(json!({ "message": "购买成功" })))
    } else {
        let _ = record_request_log(
            &state.pool,
            "POST",
            "/shop/purchase",
            Some(auth_user.qq()),
            Some("库存不足".to_string()),
            StatusCode::OK.as_u16() as i32,
        )
        .await;
        Ok(Json(json!({ "message": "库存不足" })))
    }
}

#[derive(Deserialize)]
pub struct UserTransactionsQuery {
    pub user_qq: String,
}

pub async fn get_user_transactions(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Query(params): Query<UserTransactionsQuery>,
) -> Result<Json<Value>, StatusCode> {
    // 用户只能查看自己的交易记录，除非有管理商品权限
    if auth_user.qq() != params.user_qq && !auth_user.has_permission("管理商品") {
        return Err(StatusCode::FORBIDDEN);
    }

    // 获取购买记录
    let purchases = match ShopService::get_user_purchases(&state.pool, &params.user_qq).await {
        Ok(logs) => logs,
        Err(e) => {
            log::error!("获取购买记录失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                &format!("/shop/transactions?user_qq={}", params.user_qq),
                Some(auth_user.qq()),
                Some(format!("获取购买记录失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 获取销售记录
    let sales = match ShopService::get_user_sales(&state.pool, &params.user_qq).await {
        Ok(logs) => logs,
        Err(e) => {
            log::error!("获取销售记录失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                &format!("/shop/transactions?user_qq={}", params.user_qq),
                Some(auth_user.qq()),
                Some(format!("获取销售记录失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        &format!("/shop/transactions?user_qq={}", params.user_qq),
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({
        "purchases": purchases,
        "sales": sales
    })))
}
