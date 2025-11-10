use crate::db::DbResult;
use crate::models::*;
use chrono::Local;
use log::{info, warn};
use sqlx::SqlitePool;

pub struct LpService;

impl LpService {
    // 提交LP申请
    pub async fn submit_lp_request(
        pool: &SqlitePool,
        upload_user_qq: &str,
        user_qq: &str,
        lp_type: i64,
        num: i32,
        reason: &str,
        picture: Option<String>,
        role: Option<String>,
    ) -> DbResult<i64> {
        let upload_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "INSERT INTO lplog (upload_time, upload_user_qq, user_qq, lp_type, num, reason, status, picture, role)
             VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(upload_time)
        .bind(upload_user_qq)
        .bind(user_qq)
        .bind(lp_type)
        .bind(num)
        .bind(reason)
        .bind(picture)
        .bind(role)
        .execute(pool)
        .await?;

        info!(
            "LP申请提交: 上传者={}, 关联用户={}, 类型={}, 数量={}, 状态=待审批",
            upload_user_qq, user_qq, lp_type, num
        );

        Ok(result.last_insert_rowid())
    }

    // 审批LP申请
    pub async fn process_lp_request(
        pool: &SqlitePool,
        id: i64,
        process_user_qq: &str,
        status: i32,
    ) -> DbResult<()> {
        let process_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let rows = sqlx::query(
            "UPDATE lplog SET status = ?, process_user_qq = ?, process_time = ? WHERE id = ?",
        )
        .bind(status)
        .bind(process_user_qq)
        .bind(process_time)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();

        if rows == 0 {
            warn!("LP审批未生效: id={} 可能不存在或已处理", id);
        } else {
            info!(
                "LP审批完成: 日志ID={}, 审批人={}, 状态={}",
                id, process_user_qq, status
            );
        }

        Ok(())
    }

    // 获取所有LP申请
    pub async fn get_all_lp_logs(pool: &SqlitePool) -> DbResult<Vec<LpLog>> {
        let logs = sqlx::query_as::<_, LpLog>(
            "SELECT id, upload_time, upload_user_qq, user_qq, process_user_qq, role,
                    lp_type, num, reason, status, picture, process_time
             FROM lplog ORDER BY upload_time DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    // 获取待处理的LP申请
    pub async fn get_pending_lp_logs(pool: &SqlitePool) -> DbResult<Vec<LpLog>> {
        let logs = sqlx::query_as::<_, LpLog>(
            "SELECT id, upload_time, upload_user_qq, user_qq, process_user_qq, role,
                    lp_type, num, reason, status, picture, process_time
             FROM lplog WHERE status = 0 ORDER BY upload_time ASC",
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    // 获取用户的LP历史
    pub async fn get_user_lp_history(pool: &SqlitePool, user_qq: &str) -> DbResult<Vec<LpLog>> {
        let logs = sqlx::query_as::<_, LpLog>(
            "SELECT id, upload_time, upload_user_qq, user_qq, process_user_qq, role,
                    lp_type, num, reason, status, picture, process_time
             FROM lplog WHERE user_qq = ? ORDER BY upload_time DESC",
        )
        .bind(user_qq)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    // 获取用户LP总数（使用视图）
    pub async fn get_user_lp_summary(
        pool: &SqlitePool,
        user_qq: &str,
    ) -> DbResult<Option<UserLpSummary>> {
        let summary = sqlx::query_as::<_, UserLpSummary>(
            "SELECT qq, nickname, total_lp, pending_count, approved_count, rejected_count
             FROM user_lp_summary WHERE qq = ?",
        )
        .bind(user_qq)
        .fetch_optional(pool)
        .await?;

        Ok(summary)
    }

    // 获取所有用户LP汇总
    pub async fn get_all_lp_summaries(pool: &SqlitePool) -> DbResult<Vec<UserLpSummary>> {
        let summaries = sqlx::query_as::<_, UserLpSummary>(
            "SELECT qq, nickname, total_lp, pending_count, approved_count, rejected_count
             FROM user_lp_summary ORDER BY total_lp DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(summaries)
    }

    // 获取所有LP类型
    pub async fn get_all_lp_types(pool: &SqlitePool) -> DbResult<Vec<LpType>> {
        let types = sqlx::query_as::<_, LpType>("SELECT id, name FROM lptype")
            .fetch_all(pool)
            .await?;

        Ok(types)
    }
}
