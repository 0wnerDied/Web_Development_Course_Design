use crate::db::{draw_lucky_winner, DbResult};
use crate::models::*;
use chrono::Local;
use sqlx::SqlitePool;

pub struct LuckyDrawService;

impl LuckyDrawService {
    // 创建抽奖活动
    pub async fn create_draw(
        pool: &SqlitePool,
        create_qq: &str,
        item_id: Option<i64>,
        fitting: Option<String>,
        num: i32,
        min_lp_require: i32,
        plan_time: &str,
        description: Option<String>,
    ) -> DbResult<i64> {
        // 如果指定了商品，需要先检查库存并扣除
        if let Some(item_id) = item_id {
            // 查询商品库存和所有者
            let item: (i64, String) =
                sqlx::query_as("SELECT count, seller FROM shopitems WHERE id = ?")
                    .bind(item_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|_| sqlx::Error::RowNotFound)?;

            let (stock, seller) = item;

            // 检查库存是否足够
            if stock < num as i64 {
                return Err(sqlx::Error::Decode(
                    format!("库存不足: 需要 {}, 实际 {}", num, stock).into(),
                ));
            }

            // 检查是否是商品所有者
            if seller != create_qq {
                return Err(sqlx::Error::Decode(
                    format!("只能使用自己的商品创建抽奖").into(),
                ));
            }

            // 开启事务
            let mut tx = pool.begin().await?;

            // 扣除库存
            let update_result =
                sqlx::query("UPDATE shopitems SET count = count - ? WHERE id = ? AND count >= ?")
                    .bind(num)
                    .bind(item_id)
                    .bind(num)
                    .execute(&mut *tx)
                    .await?;

            // 检查是否成功更新（防止并发问题）
            if update_result.rows_affected() == 0 {
                return Err(sqlx::Error::Decode("库存不足或商品不存在".into()));
            }

            // 创建抽奖记录
            let create_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let result = sqlx::query(
                "INSERT INTO luckydrawlog (create_time, create_qq, item_id, fitting, num, min_lp_require,
                                           plan_time, status, description)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)",
            )
            .bind(create_time)
            .bind(create_qq)
            .bind(Some(item_id))
            .bind(fitting)
            .bind(num)
            .bind(min_lp_require)
            .bind(plan_time)
            .bind(description)
            .execute(&mut *tx)
            .await?;

            let draw_id = result.last_insert_rowid();

            // 提交事务
            tx.commit().await?;

            log::info!(
                "创建抽奖 ID={}, 扣除商品 ID={} 库存 {} 个",
                draw_id,
                item_id,
                num
            );
            Ok(draw_id)
        } else {
            // 没有指定商品，直接创建抽奖
            let create_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let result = sqlx::query(
                "INSERT INTO luckydrawlog (create_time, create_qq, item_id, fitting, num, min_lp_require,
                                           plan_time, status, description)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)",
            )
            .bind(create_time)
            .bind(create_qq)
            .bind(item_id)
            .bind(fitting)
            .bind(num)
            .bind(min_lp_require)
            .bind(plan_time)
            .bind(description)
            .execute(pool)
            .await?;

            Ok(result.last_insert_rowid())
        }
    }

    // 执行抽奖（调用存储过程）
    pub async fn execute_draw(pool: &SqlitePool, draw_id: i64) -> DbResult<Option<Vec<String>>> {
        draw_lucky_winner(pool, draw_id).await
    }

    // 手动设置中奖者
    pub async fn set_winner(pool: &SqlitePool, draw_id: i64, winner_qq: &str) -> DbResult<()> {
        sqlx::query("UPDATE luckydrawlog SET status = 1, winner_qq = ? WHERE id = ?")
            .bind(winner_qq)
            .bind(draw_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // 获取所有抽奖活动
    pub async fn get_all_draws(pool: &SqlitePool) -> DbResult<Vec<LuckyDrawLog>> {
        let draws = sqlx::query_as::<_, LuckyDrawLog>(
            "SELECT id, create_time, create_qq, item_id, fitting, num, min_lp_require,
                    plan_time, status, winner_qq, description
             FROM luckydrawlog ORDER BY create_time DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(draws)
    }

    // 获取待开奖的活动
    pub async fn get_pending_draws(pool: &SqlitePool) -> DbResult<Vec<LuckyDrawLog>> {
        let draws = sqlx::query_as::<_, LuckyDrawLog>(
            "SELECT id, create_time, create_qq, item_id, fitting, num, min_lp_require,
                    plan_time, status, winner_qq, description
             FROM luckydrawlog WHERE status = 0 ORDER BY plan_time ASC",
        )
        .fetch_all(pool)
        .await?;

        Ok(draws)
    }

    // 获取用户中奖记录
    pub async fn get_user_wins(pool: &SqlitePool, user_qq: &str) -> DbResult<Vec<LuckyDrawLog>> {
        let draws = sqlx::query_as::<_, LuckyDrawLog>(
            "SELECT id, create_time, create_qq, item_id, fitting, num, min_lp_require,
                    plan_time, status, winner_qq, description
             FROM luckydrawlog WHERE winner_qq = ? ORDER BY create_time DESC",
        )
        .bind(user_qq)
        .fetch_all(pool)
        .await?;

        Ok(draws)
    }

    // 删除抽奖活动
    pub async fn delete_draw(pool: &SqlitePool, draw_id: i64) -> DbResult<()> {
        // 开启事务
        let mut tx = pool.begin().await?;

        // 查询抽奖信息（是否已开奖、关联的商品ID和数量）
        let draw_info: (i32, Option<i64>, i32) =
            sqlx::query_as("SELECT status, item_id, num FROM luckydrawlog WHERE id = ?")
                .bind(draw_id)
                .fetch_one(&mut *tx)
                .await?;

        let (status, item_id, num) = draw_info;

        // 如果未开奖且关联了商品，需要恢复库存
        if status == 0 && item_id.is_some() {
            let item_id = item_id.unwrap();
            sqlx::query("UPDATE shopitems SET count = count + ? WHERE id = ?")
                .bind(num)
                .bind(item_id)
                .execute(&mut *tx)
                .await?;

            log::info!(
                "删除抽奖 ID={}, 恢复商品 ID={} 库存 {} 个",
                draw_id,
                item_id,
                num
            );
        }

        // 删除抽奖记录
        sqlx::query("DELETE FROM luckydrawlog WHERE id = ?")
            .bind(draw_id)
            .execute(&mut *tx)
            .await?;

        // 提交事务
        tx.commit().await?;

        Ok(())
    }
}
