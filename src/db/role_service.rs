use crate::db::DbResult;
use crate::models::Role;
use sqlx::SqlitePool;

pub struct RoleService;

impl RoleService {
    /// 获取所有角色
    pub async fn get_all_roles(pool: &SqlitePool) -> DbResult<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT role_id, name, description FROM role ORDER BY role_id",
        )
        .fetch_all(pool)
        .await?;

        Ok(roles)
    }

    /// 创建新角色
    pub async fn create_role(
        pool: &SqlitePool,
        name: &str,
        description: Option<&str>,
    ) -> DbResult<i64> {
        let result = sqlx::query("INSERT INTO role (name, description) VALUES (?, ?)")
            .bind(name)
            .bind(description)
            .execute(pool)
            .await?;

        Ok(result.last_insert_rowid())
    }

    /// 根据名称获取角色
    pub async fn get_role_by_name(pool: &SqlitePool, name: &str) -> DbResult<Option<Role>> {
        let role =
            sqlx::query_as::<_, Role>("SELECT role_id, name, description FROM role WHERE name = ?")
                .bind(name)
                .fetch_optional(pool)
                .await?;

        Ok(role)
    }

    /// 获取用户主角色
    pub async fn get_user_role(pool: &SqlitePool, user_qq: &str) -> DbResult<Option<Role>> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT r.role_id, r.name, r.description
             FROM user u
             JOIN role r ON u.main_role_id = r.role_id
             WHERE u.qq = ?",
        )
        .bind(user_qq)
        .fetch_optional(pool)
        .await?;

        Ok(role)
    }

    /// 设置用户主角色
    pub async fn assign_main_role(pool: &SqlitePool, user_qq: &str, role_id: i64) -> DbResult<()> {
        sqlx::query("UPDATE user SET main_role_id = ? WHERE qq = ?")
            .bind(role_id)
            .bind(user_qq)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 给角色分配权限
    pub async fn grant_permission_to_role(
        pool: &SqlitePool,
        role_id: i64,
        permission_name: &str,
    ) -> DbResult<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO rolepermissionlink (role_id, permission_name) VALUES (?, ?)",
        )
        .bind(role_id)
        .bind(permission_name)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 从角色移除权限
    pub async fn revoke_permission_from_role(
        pool: &SqlitePool,
        role_id: i64,
        permission_name: &str,
    ) -> DbResult<()> {
        sqlx::query("DELETE FROM rolepermissionlink WHERE role_id = ? AND permission_name = ?")
            .bind(role_id)
            .bind(permission_name)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// 获取角色的所有权限
    pub async fn get_role_permissions(pool: &SqlitePool, role_id: i64) -> DbResult<Vec<String>> {
        let permissions = sqlx::query_scalar::<_, String>(
            "SELECT permission_name FROM rolepermissionlink WHERE role_id = ? ORDER BY permission_name",
        )
        .bind(role_id)
        .fetch_all(pool)
        .await?;

        Ok(permissions)
    }

    /// 删除角色
    pub async fn delete_role(pool: &SqlitePool, role_id: i64) -> DbResult<()> {
        // 检查是否是系统核心角色（管理员或成员）
        let role_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM role WHERE role_id = ?")
                .bind(role_id)
                .fetch_optional(pool)
                .await?;

        if let Some(name) = role_name {
            if name == "管理员" || name == "成员" {
                return Err(sqlx::Error::Decode(
                    format!("不能删除系统核心角色: {}", name).into(),
                ));
            }
        }

        // 查询"成员"角色的ID
        let member_role_id: Option<i64> =
            sqlx::query_scalar("SELECT role_id FROM role WHERE name = ?")
                .bind("成员")
                .fetch_optional(pool)
                .await?;

        let member_role_id =
            member_role_id.ok_or_else(|| sqlx::Error::Decode("未找到'成员'角色".into()))?;

        // 将使用此角色的用户的main_role_id设为"成员"角色
        sqlx::query("UPDATE user SET main_role_id = ? WHERE main_role_id = ?")
            .bind(member_role_id)
            .bind(role_id)
            .execute(pool)
            .await?;

        // 删除角色（级联删除会自动删除rolepermissionlink中的记录）
        sqlx::query("DELETE FROM role WHERE role_id = ?")
            .bind(role_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
