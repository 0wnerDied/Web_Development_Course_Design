use crate::db::DbResult;
use crate::models::*;
use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::SqlitePool;

pub struct UserService;

impl UserService {
    // 用户注册
    pub async fn register(
        pool: &SqlitePool,
        qq: &str,
        nickname: &str,
        password: &str,
        birthday: Option<&str>,
    ) -> DbResult<()> {
        let hashed_password = hash(password, DEFAULT_COST).expect("密码加密失败");

        let default_role_id: Option<i64> =
            sqlx::query_scalar("SELECT role_id FROM role WHERE name = ?")
                .bind("成员")
                .fetch_optional(pool)
                .await?;

        sqlx::query(
            "INSERT INTO user (qq, main_role_id, nickname, password, birthday) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(qq)
        .bind(default_role_id)
        .bind(nickname)
        .bind(hashed_password)
        .bind(birthday)
        .execute(pool)
        .await?;

        Ok(())
    }

    // 用户登录
    pub async fn login(pool: &SqlitePool, qq: &str, password: &str) -> DbResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT qq, main_role_id, nickname, password, birthday FROM user WHERE qq = ?",
        )
        .bind(qq)
        .fetch_optional(pool)
        .await?;

        if let Some(user) = user {
            if verify(password, &user.password).unwrap_or(false) {
                return Ok(Some(user));
            }
        }

        Ok(None)
    }

    // 获取单个用户
    pub async fn get_user(pool: &SqlitePool, qq: &str) -> DbResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT qq, main_role_id, nickname, password, birthday FROM user WHERE qq = ?",
        )
        .bind(qq)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    // 获取所有用户
    pub async fn get_all_users(pool: &SqlitePool) -> DbResult<Vec<UserWithRole>> {
        let users = sqlx::query_as::<_, UserWithRole>(
            "SELECT u.qq, u.main_role_id, u.nickname, u.password, u.birthday, r.name as role_name 
             FROM user u 
             LEFT JOIN role r ON u.main_role_id = r.role_id",
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    // 更新用户信息
    pub async fn update_user(
        pool: &SqlitePool,
        qq: &str,
        nickname: Option<String>,
        birthday: Option<String>,
    ) -> DbResult<()> {
        if let Some(nick) = nickname {
            sqlx::query("UPDATE user SET nickname = ? WHERE qq = ?")
                .bind(nick)
                .bind(qq)
                .execute(pool)
                .await?;
        }

        if let Some(birth) = birthday {
            sqlx::query("UPDATE user SET birthday = ? WHERE qq = ?")
                .bind(birth)
                .bind(qq)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    // 修改密码
    pub async fn change_password(
        pool: &SqlitePool,
        qq: &str,
        old_password: &str,
        new_password: &str,
    ) -> DbResult<bool> {
        let current_password: Option<String> =
            sqlx::query_scalar("SELECT password FROM user WHERE qq = ?")
                .bind(qq)
                .fetch_optional(pool)
                .await?;

        let Some(current_password) = current_password else {
            return Ok(false);
        };

        if !verify(old_password, &current_password).unwrap_or(false) {
            return Ok(false);
        }

        let hashed_new_password = hash(new_password, DEFAULT_COST).expect("密码加密失败");
        sqlx::query("UPDATE user SET password = ? WHERE qq = ?")
            .bind(hashed_new_password)
            .bind(qq)
            .execute(pool)
            .await?;

        Ok(true)
    }

    // 删除用户
    pub async fn delete_user(pool: &SqlitePool, qq: &str) -> DbResult<()> {
        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM requestlog WHERE user_qq = ?")
            .bind(qq)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM shoplog WHERE buyer = ? OR seller = ?")
            .bind(qq)
            .bind(qq)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM shopitems WHERE seller = ?")
            .bind(qq)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM luckydrawlog WHERE create_qq = ? OR winner_qq = ?")
            .bind(qq)
            .bind(qq)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "DELETE FROM lplog WHERE upload_user_qq = ? OR user_qq = ? OR process_user_qq = ?",
        )
        .bind(qq)
        .bind(qq)
        .bind(qq)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM user WHERE qq = ?")
            .bind(qq)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    // 搜索用户
    pub async fn search_users(pool: &SqlitePool, keyword: &str) -> DbResult<Vec<User>> {
        let pattern = format!("%{}%", keyword);
        let users = sqlx::query_as::<_, User>(
            "SELECT qq, main_role_id, nickname, password, birthday FROM user
             WHERE qq LIKE ? OR nickname LIKE ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }
}
