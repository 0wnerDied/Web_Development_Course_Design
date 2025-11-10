use chrono::Local;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::time::interval;

/// 定时检查并自动开奖
pub async fn start_lottery_scheduler(pool: SqlitePool) {
    tokio::spawn(async move {
        // 每分钟检查一次
        let mut ticker = interval(Duration::from_secs(60));

        loop {
            ticker.tick().await;

            if let Err(e) = check_and_execute_pending_lotteries(&pool).await {
                tracing::error!("定时开奖任务执行失败: {}", e);
            }
        }
    });
}

/// 检查并执行到期的抽奖
async fn check_and_execute_pending_lotteries(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let current_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 查询所有到期但未开奖的抽奖
    let pending_draws = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM luckydrawlog 
         WHERE status = 0 
         AND plan_time <= ? 
         ORDER BY plan_time ASC",
    )
    .bind(&current_time)
    .fetch_all(pool)
    .await?;

    if !pending_draws.is_empty() {
        tracing::info!("发现 {} 个到期待开奖的抽奖", pending_draws.len());
    }

    // 逐个执行开奖
    for (draw_id,) in pending_draws {
        match team_operation_system::db::draw_lucky_winner(pool, draw_id).await {
            Ok(Some(winners)) => {
                tracing::info!(
                    "自动开奖成功: 抽奖ID={}, 中奖者={:?} (共{}人)",
                    draw_id,
                    winners,
                    winners.len()
                );
            }
            Ok(None) => {
                tracing::warn!("自动开奖失败: 抽奖ID={}, 没有符合条件的参与者", draw_id);
                // 标记为已处理，避免重复检查
                let _ = sqlx::query("UPDATE luckydrawlog SET status = 2 WHERE id = ?")
                    .bind(draw_id)
                    .execute(pool)
                    .await;
            }
            Err(e) => {
                tracing::error!("自动开奖出错: 抽奖ID={}, 错误={}", draw_id, e);
            }
        }
    }

    Ok(())
}
