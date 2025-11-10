mod log_service;
mod lp_service;
mod lucky_draw_service;
mod permission_service;
mod role_service;
mod shop_service;
mod user_service;

pub use log_service::*;
pub use lp_service::LpService;
pub use lucky_draw_service::LuckyDrawService;
pub use permission_service::PermissionService;
pub use role_service::RoleService;
pub use shop_service::ShopService;
pub use user_service::UserService;

use chrono::Local;
use sqlx::{Executor, SqlitePool};

pub type DbPool = SqlitePool;
pub type DbResult<T> = Result<T, sqlx::Error>;

pub async fn init_database(pool: &SqlitePool) -> DbResult<()> {
    pool.execute("PRAGMA foreign_keys = ON").await?;
    tracing::info!("已启用外键约束");

    pool.execute(
        "CREATE TABLE IF NOT EXISTS role (
            role_id INTEGER PRIMARY KEY AUTOINCREMENT,
            name VARCHAR NOT NULL UNIQUE,
            description VARCHAR
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS user (
            qq VARCHAR PRIMARY KEY NOT NULL,
            main_role_id INTEGER,
            nickname VARCHAR NOT NULL,
            password VARCHAR NOT NULL,
            birthday VARCHAR,
            FOREIGN KEY(main_role_id) REFERENCES role(role_id)
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS permission (
            name VARCHAR PRIMARY KEY NOT NULL
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS rolepermissionlink (
            role_id INTEGER NOT NULL,
            permission_name VARCHAR NOT NULL,
            PRIMARY KEY (role_id, permission_name),
            FOREIGN KEY(role_id) REFERENCES role(role_id) ON DELETE CASCADE,
            FOREIGN KEY(permission_name) REFERENCES permission(name) ON DELETE CASCADE
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS lptype (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name VARCHAR NOT NULL UNIQUE
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS lplog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            upload_time VARCHAR NOT NULL,
            upload_user_qq VARCHAR NOT NULL,
            user_qq VARCHAR NOT NULL,
            process_user_qq VARCHAR,
            role VARCHAR,
            lp_type INTEGER NOT NULL,
            num INTEGER NOT NULL,
            reason VARCHAR NOT NULL,
            status INTEGER NOT NULL DEFAULT 0,
            picture VARCHAR,
            process_time VARCHAR,
            FOREIGN KEY(lp_type) REFERENCES lptype(id),
            FOREIGN KEY(process_user_qq) REFERENCES user(qq),
            FOREIGN KEY(user_qq) REFERENCES user(qq),
            FOREIGN KEY(upload_user_qq) REFERENCES user(qq)
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS luckydrawlog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            create_time VARCHAR NOT NULL,
            create_qq VARCHAR NOT NULL,
            item_id INTEGER,
            fitting TEXT,
            num INTEGER NOT NULL,
            min_lp_require INTEGER NOT NULL,
            plan_time VARCHAR NOT NULL,
            status INTEGER NOT NULL DEFAULT 0,
            winner_qq VARCHAR,
            description VARCHAR,
            FOREIGN KEY(create_qq) REFERENCES user(qq),
            FOREIGN KEY(item_id) REFERENCES shopitems(id) ON DELETE SET NULL
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS shopitems (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            count INTEGER NOT NULL,
            price VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            seller VARCHAR NOT NULL,
            location VARCHAR NOT NULL,
            FOREIGN KEY(seller) REFERENCES user(qq)
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS shoplog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            buyer VARCHAR NOT NULL,
            count INTEGER NOT NULL,
            price VARCHAR NOT NULL,
            name VARCHAR NOT NULL,
            time VARCHAR NOT NULL,
            seller VARCHAR NOT NULL,
            location VARCHAR NOT NULL,
            FOREIGN KEY(seller) REFERENCES user(qq),
            FOREIGN KEY(buyer) REFERENCES user(qq)
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS requestlog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            method VARCHAR NOT NULL,
            path VARCHAR NOT NULL,
            user_qq VARCHAR,
            body TEXT,
            status INTEGER NOT NULL,
            timestamp VARCHAR NOT NULL,
            FOREIGN KEY(user_qq) REFERENCES user(qq)
        )",
    )
    .await?;

    pool.execute(
        "CREATE VIEW IF NOT EXISTS user_lp_summary AS
        SELECT 
            u.qq,
            u.nickname,
            COALESCE(SUM(CASE WHEN l.status = 1 THEN l.num ELSE 0 END), 0) as total_lp,
            COUNT(CASE WHEN l.status = 0 THEN 1 END) as pending_count,
            COUNT(CASE WHEN l.status = 1 THEN 1 END) as approved_count,
            COUNT(CASE WHEN l.status = 2 THEN 1 END) as rejected_count
        FROM user u
        LEFT JOIN lplog l ON u.qq = l.user_qq
        GROUP BY u.qq, u.nickname",
    )
    .await?;

    pool.execute(
        "CREATE TRIGGER IF NOT EXISTS shoplog_auto_time
        AFTER INSERT ON shoplog
        FOR EACH ROW
        WHEN NEW.time IS NULL OR NEW.time = ''
        BEGIN
            UPDATE shoplog SET time = datetime('now', 'localtime') WHERE id = NEW.id;
        END",
    )
    .await?;

    pool.execute(
        "CREATE TRIGGER IF NOT EXISTS update_shop_inventory
        AFTER INSERT ON shoplog
        FOR EACH ROW
        BEGIN
            UPDATE shopitems 
            SET count = count - NEW.count 
            WHERE id = (SELECT id FROM shopitems WHERE name = NEW.name AND seller = NEW.seller LIMIT 1)
            AND count >= NEW.count;
        END",
    )
    .await?;

    // 索引，优化查询性能

    // lplog 表索引 - LP记录查询优化
    pool.execute("CREATE INDEX IF NOT EXISTS idx_lplog_status ON lplog(status)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_lplog_user_qq ON lplog(user_qq)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_lplog_upload_user ON lplog(upload_user_qq)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_lplog_process_user ON lplog(process_user_qq)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_lplog_upload_time ON lplog(upload_time)")
        .await?;
    // 复合索引：按状态筛选并按时间排序（批量审批场景）
    pool.execute(
        "CREATE INDEX IF NOT EXISTS idx_lplog_status_time ON lplog(status, upload_time DESC)",
    )
    .await?;
    // 复合索引：按用户和状态查询（用户LP历史场景）
    pool.execute("CREATE INDEX IF NOT EXISTS idx_lplog_user_status ON lplog(user_qq, status)")
        .await?;

    // shoplog 表索引 - 商店交易记录查询优化
    pool.execute("CREATE INDEX IF NOT EXISTS idx_shoplog_buyer ON shoplog(buyer)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_shoplog_seller ON shoplog(seller)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_shoplog_time ON shoplog(time DESC)")
        .await?;
    // 复合索引：按买家查询并按时间排序
    pool.execute("CREATE INDEX IF NOT EXISTS idx_shoplog_buyer_time ON shoplog(buyer, time DESC)")
        .await?;

    // luckydrawlog 表索引 - 抽奖记录查询优化
    pool.execute("CREATE INDEX IF NOT EXISTS idx_luckydraw_status ON luckydrawlog(status)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_luckydraw_create_qq ON luckydrawlog(create_qq)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_luckydraw_winner ON luckydrawlog(winner_qq)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_luckydraw_plan_time ON luckydrawlog(plan_time)")
        .await?;
    // 复合索引：按状态和计划时间查询（定时抽奖场景）
    pool.execute(
        "CREATE INDEX IF NOT EXISTS idx_luckydraw_status_plan ON luckydrawlog(status, plan_time)",
    )
    .await?;

    // requestlog 表索引 - 请求日志查询优化
    pool.execute("CREATE INDEX IF NOT EXISTS idx_requestlog_user ON requestlog(user_qq)")
        .await?;
    pool.execute(
        "CREATE INDEX IF NOT EXISTS idx_requestlog_timestamp ON requestlog(timestamp DESC)",
    )
    .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_requestlog_status ON requestlog(status)")
        .await?;
    // 复合索引：按用户和时间查询日志（日志分页场景）
    pool.execute("CREATE INDEX IF NOT EXISTS idx_requestlog_user_time ON requestlog(user_qq, timestamp DESC)")
        .await?;

    // shopitems 表索引 - 商品查询优化
    pool.execute("CREATE INDEX IF NOT EXISTS idx_shopitems_seller ON shopitems(seller)")
        .await?;
    pool.execute("CREATE INDEX IF NOT EXISTS idx_shopitems_name ON shopitems(name)")
        .await?;

    let default_permissions = vec![
        "审核LP",
        "发起抽奖",
        "管理商品",
        "用户管理",
        "管理角色",
        "查看日志",
    ];

    for perm in default_permissions {
        sqlx::query("INSERT OR IGNORE INTO permission (name) VALUES (?)")
            .bind(perm)
            .execute(pool)
            .await?;
    }

    let default_roles = vec![
        ("管理员", "系统全面管理权限"),
        ("审核员", "负责LP审批与抽奖管理"),
        ("成员", "日常使用权限"),
    ];

    for (name, desc) in default_roles {
        sqlx::query("INSERT OR IGNORE INTO role (name, description) VALUES (?, ?)")
            .bind(name)
            .bind(desc)
            .execute(pool)
            .await?;
    }

    let default_lp_types = vec!["奖励", "惩罚", "兑换", "调整"];

    for lp_type in default_lp_types {
        sqlx::query("INSERT OR IGNORE INTO lptype (name) VALUES (?)")
            .bind(lp_type)
            .execute(pool)
            .await?;
    }

    // 创建默认管理员用户
    let admin_role_id: Option<i64> = sqlx::query_scalar("SELECT role_id FROM role WHERE name = ?")
        .bind("管理员")
        .fetch_optional(pool)
        .await?;

    if let Some(role_id) = admin_role_id {
        // 默认管理员信息
        let default_admin_qq = "9999"; // QQ 号默认最低五位，这里是占位，建议修改为自己的 QQ
        let default_admin_nickname = "管理员";
        let default_admin_password = "admin@666"; // 默认密码

        // 检查管理员是否已存在
        let admin_exists: Option<String> = sqlx::query_scalar("SELECT qq FROM user WHERE qq = ?")
            .bind(default_admin_qq)
            .fetch_optional(pool)
            .await?;

        if admin_exists.is_none() {
            // 使用 bcrypt 加密默认密码
            use bcrypt::{hash, DEFAULT_COST};
            let hashed_password = hash(default_admin_password, DEFAULT_COST).expect("密码加密失败");

            sqlx::query(
                "INSERT INTO user (qq, main_role_id, nickname, password, birthday) VALUES (?, ?, ?, ?, NULL)",
            )
            .bind(default_admin_qq)
            .bind(role_id)
            .bind(default_admin_nickname)
            .bind(hashed_password)
            .execute(pool)
            .await?;

            // 给管理员角色分配所有权限
            let all_permissions: Vec<String> = sqlx::query_scalar("SELECT name FROM permission")
                .fetch_all(pool)
                .await?;

            for perm in all_permissions {
                sqlx::query(
                    "INSERT OR IGNORE INTO rolepermissionlink (role_id, permission_name) VALUES (?, ?)",
                )
                .bind(role_id)
                .bind(&perm)
                .execute(pool)
                .await?;
            }

            tracing::info!("已创建默认管理员用户");
            tracing::info!("   账号: {}", default_admin_qq);
            tracing::info!("   密码: {}", default_admin_password);
            tracing::info!("   请在首次登录后立即修改密码！");
        }
    }

    Ok(())
}

pub async fn batch_approve_lp(
    pool: &SqlitePool,
    ids: &[i64],
    process_qq: &str,
    status: i32,
) -> DbResult<u64> {
    let process_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut approved: u64 = 0;

    let mut tx = pool.begin().await?;

    for id in ids {
        let result = sqlx::query(
            "UPDATE lplog SET status = ?, process_user_qq = ?, process_time = ? \
             WHERE id = ? AND status = 0",
        )
        .bind(status)
        .bind(process_qq)
        .bind(&process_time)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        approved += result.rows_affected();
    }

    tx.commit().await?;

    Ok(approved)
}

pub async fn draw_lucky_winner(pool: &SqlitePool, draw_id: i64) -> DbResult<Option<Vec<String>>> {
    // 查询抽奖信息，包括关联的商品ID和数量
    let record = sqlx::query_as::<_, (i64, i32, Option<i64>, i32)>(
        "SELECT min_lp_require, status, item_id, num FROM luckydrawlog WHERE id = ?",
    )
    .bind(draw_id)
    .fetch_optional(pool)
    .await?;

    let Some((min_lp, status, item_id, num)) = record else {
        return Ok(None);
    };

    if status != 0 {
        return Ok(None);
    }

    // 获取所有符合条件的用户
    let eligible_users =
        sqlx::query_scalar::<_, String>("SELECT qq FROM user_lp_summary WHERE total_lp >= ?")
            .bind(min_lp)
            .fetch_all(pool)
            .await?;

    // 如果没有符合条件的用户，返回 None
    if eligible_users.is_empty() {
        return Ok(None);
    }

    // 使用时间戳和抽奖ID作为随机种子选择中奖者
    use rand::seq::SliceRandom;
    use rand::SeedableRng;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
        ^ draw_id as u64;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // 根据数量选择多个中奖者（不重复）
    let winner_count = std::cmp::min(num as usize, eligible_users.len());
    let mut shuffled = eligible_users.clone();
    shuffled.shuffle(&mut rng);
    let winners: Vec<String> = shuffled.into_iter().take(winner_count).collect();

    if !winners.is_empty() {
        // 将多个中奖者用逗号连接存储
        let winners_str = winners.join(", ");

        // 更新抽奖状态为已开奖
        sqlx::query("UPDATE luckydrawlog SET status = 1, winner_qq = ? WHERE id = ?")
            .bind(&winners_str)
            .bind(draw_id)
            .execute(pool)
            .await?;

        tracing::info!("抽奖 {} 开奖成功，中奖者: {}", draw_id, winners_str);
        Ok(Some(winners))
    } else {
        Ok(None)
    }
}

pub async fn record_request_log(
    pool: &SqlitePool,
    method: &str,
    path: &str,
    user_qq: Option<&str>,
    body: Option<String>,
    status: i32,
) -> DbResult<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let user_value = user_qq.unwrap_or("");
    let body_value = body.unwrap_or_default();

    sqlx::query(
        "INSERT INTO requestlog (method, path, user_qq, body, status, timestamp) VALUES (?, ?, NULLIF(?, ''), ?, ?, ?)",
    )
    .bind(method)
    .bind(path)
    .bind(user_value)
    .bind(body_value)
    .bind(status)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(())
}
