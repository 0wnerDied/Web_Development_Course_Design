use crate::db::DbResult;
use crate::models::*;
use sqlx::SqlitePool;

pub struct PermissionService;

impl PermissionService {
    /// 获取用户的所有权限（通过角色获得）
    pub async fn get_user_permissions(pool: &SqlitePool, user_qq: &str) -> DbResult<Vec<String>> {
        let permissions = sqlx::query_scalar::<_, String>(
            "SELECT rpl.permission_name 
             FROM user u
             JOIN rolepermissionlink rpl ON u.main_role_id = rpl.role_id
             WHERE u.qq = ?
             ORDER BY rpl.permission_name",
        )
        .bind(user_qq)
        .fetch_all(pool)
        .await?;

        Ok(permissions)
    }

    /// 获取所有权限
    pub async fn get_all_permissions(pool: &SqlitePool) -> DbResult<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>("SELECT name FROM permission")
            .fetch_all(pool)
            .await?;

        Ok(permissions)
    }
}
