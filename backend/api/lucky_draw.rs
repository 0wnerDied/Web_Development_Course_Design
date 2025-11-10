use crate::auth::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use team_operation_system::db::{record_request_log, LuckyDrawService};

#[derive(Deserialize)]
pub struct CreateDrawRequest {
    pub create_qq: String,
    pub item_id: Option<i64>,
    pub fitting: Option<String>,
    pub num: i32,
    pub min_lp_require: i32,
    pub plan_time: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct SetWinnerRequest {
    pub winner_qq: String,
}

pub async fn list_draws(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    if !auth_user.has_permission("发起抽奖") && !auth_user.has_permission("查看日志") {
        return Err(StatusCode::FORBIDDEN);
    }

    let draws = match LuckyDrawService::get_all_draws(&state.pool).await {
        Ok(draws) => draws,
        Err(e) => {
            log::error!("获取抽奖列表失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "GET",
                "/lucky-draw",
                Some(auth_user.qq()),
                Some(format!("获取抽奖失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let _ = record_request_log(
        &state.pool,
        "GET",
        "/lucky-draw",
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "draws": draws })))
}

pub async fn create_draw(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateDrawRequest>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("发起抽奖")?;
    if auth_user.qq() != payload.create_qq {
        return Err(StatusCode::FORBIDDEN);
    }

    let CreateDrawRequest {
        create_qq,
        item_id,
        fitting,
        num,
        min_lp_require,
        plan_time,
        description,
    } = payload;

    let id = match LuckyDrawService::create_draw(
        &state.pool,
        &create_qq,
        item_id,
        fitting.clone(),
        num,
        min_lp_require,
        &plan_time,
        description.clone(),
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("创建抽奖失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                "/lucky-draw/create",
                Some(auth_user.qq()),
                Some(format!("创建抽奖失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    log::info!("创建抽奖活动: ID={}, 创建人={}", id, create_qq);
    let _ = record_request_log(
        &state.pool,
        "POST",
        "/lucky-draw/create",
        Some(auth_user.qq()),
        Some(
            serde_json::to_string(&json!({
                "item_id": item_id,
                "fitting": fitting,
                "num": num,
                "min_lp_require": min_lp_require,
                "plan_time": plan_time,
                "description": description,
            }))
            .unwrap_or_default(),
        ),
        StatusCode::OK.as_u16() as i32,
    )
    .await;
    Ok(Json(json!({ "message": "抽奖活动创建成功", "id": id })))
}

pub async fn execute_draw(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("发起抽奖")?;
    let winner = match LuckyDrawService::execute_draw(&state.pool, id).await {
        Ok(winner) => winner,
        Err(e) => {
            log::error!("执行抽奖失败: {}", e);
            let _ = record_request_log(
                &state.pool,
                "POST",
                &format!("/lucky-draw/execute/{}", id),
                Some(auth_user.qq()),
                Some(format!("执行抽奖失败: {}", e)),
                StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
            )
            .await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match winner {
        Some(winners) => {
            log::info!("抽奖活动 {} 开奖，中奖者: {:?}", id, winners);
            let _ = record_request_log(
                &state.pool,
                "POST",
                &format!("/lucky-draw/execute/{}", id),
                Some(auth_user.qq()),
                Some(json!({ "winners": winners.clone() }).to_string()),
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({
                "message": "开奖成功",
                "winners": winners,
                "count": winners.len()
            })))
        }
        None => {
            let _ = record_request_log(
                &state.pool,
                "POST",
                &format!("/lucky-draw/execute/{}", id),
                Some(auth_user.qq()),
                None,
                StatusCode::OK.as_u16() as i32,
            )
            .await;
            Ok(Json(json!({ "message": "没有符合条件的参与者" })))
        }
    }
}

pub async fn set_manual_winner(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<SetWinnerRequest>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("发起抽奖")?;
    if let Err(e) = LuckyDrawService::set_winner(&state.pool, id, &payload.winner_qq).await {
        log::error!("设置中奖者失败: {}", e);
        let _ = record_request_log(
            &state.pool,
            "POST",
            &format!("/lucky-draw/winner/{}", id),
            Some(auth_user.qq()),
            Some(format!("设置中奖者失败: {}", e)),
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let _ = record_request_log(
        &state.pool,
        "POST",
        &format!("/lucky-draw/winner/{}", id),
        Some(auth_user.qq()),
        Some(serde_json::to_string(&json!({ "winner_qq": payload.winner_qq })).unwrap_or_default()),
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "message": "中奖者设置成功" })))
}

pub async fn delete_draw(
    auth_user: AuthenticatedUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, StatusCode> {
    auth_user.require_permission("发起抽奖")?;

    if let Err(e) = LuckyDrawService::delete_draw(&state.pool, id).await {
        log::error!("删除抽奖失败: {}", e);
        let _ = record_request_log(
            &state.pool,
            "DELETE",
            &format!("/lucky-draw/{}", id),
            Some(auth_user.qq()),
            Some(format!("删除抽奖失败: {}", e)),
            StatusCode::INTERNAL_SERVER_ERROR.as_u16() as i32,
        )
        .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    log::info!("删除抽奖活动: ID={}, 操作人={}", id, auth_user.qq());
    let _ = record_request_log(
        &state.pool,
        "DELETE",
        &format!("/lucky-draw/{}", id),
        Some(auth_user.qq()),
        None,
        StatusCode::OK.as_u16() as i32,
    )
    .await;

    Ok(Json(json!({ "message": "抽奖活动删除成功" })))
}
