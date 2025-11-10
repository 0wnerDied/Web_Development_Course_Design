use crate::db::DbResult;
use crate::models::*;
use chrono::Local;
use sqlx::SqlitePool;

pub struct ShopService;

impl ShopService {
    // 上架商品
    pub async fn add_item(
        pool: &SqlitePool,
        count: i32,
        price: &str,
        name: &str,
        seller: &str,
        location: &str,
    ) -> DbResult<i64> {
        let result = sqlx::query(
            "INSERT INTO shopitems (count, price, name, seller, location) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(count)
        .bind(price)
        .bind(name)
        .bind(seller)
        .bind(location)
        .execute(pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    // 购买商品
    pub async fn purchase_item(
        pool: &SqlitePool,
        buyer: &str,
        item_id: i64,
        count: i32,
    ) -> DbResult<bool> {
        let mut tx = pool.begin().await?;

        let item = sqlx::query_as::<_, ShopItem>(
            "SELECT id, count, price, name, seller, location FROM shopitems WHERE id = ?",
        )
        .bind(item_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(item) = item else {
            tx.rollback().await?;
            return Ok(false);
        };

        if item.count < count {
            tx.rollback().await?;
            return Ok(false);
        }

        let time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "INSERT INTO shoplog (buyer, count, price, name, time, seller, location)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(buyer)
        .bind(count)
        .bind(&item.price)
        .bind(&item.name)
        .bind(&time)
        .bind(&item.seller)
        .bind(&item.location)
        .execute(&mut *tx)
        .await?;

        let remaining = sqlx::query_scalar::<_, i32>("SELECT count FROM shopitems WHERE id = ?")
            .bind(item_id)
            .fetch_optional(&mut *tx)
            .await?;

        if remaining.unwrap_or(0) <= 0 {
            sqlx::query("DELETE FROM shopitems WHERE id = ?")
                .bind(item_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(true)
    }

    // 获取所有在售商品
    pub async fn get_all_items(pool: &SqlitePool) -> DbResult<Vec<ShopItem>> {
        let items = sqlx::query_as::<_, ShopItem>(
            "SELECT id, count, price, name, seller, location FROM shopitems WHERE count > 0",
        )
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    // 获取用户的商品
    pub async fn get_user_items(pool: &SqlitePool, seller: &str) -> DbResult<Vec<ShopItem>> {
        let items = sqlx::query_as::<_, ShopItem>(
            "SELECT id, count, price, name, seller, location FROM shopitems WHERE seller = ?",
        )
        .bind(seller)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    // 修改商品信息
    pub async fn update_item(
        pool: &SqlitePool,
        item_id: i64,
        count: Option<i32>,
        price: Option<String>,
        location: Option<String>,
    ) -> DbResult<()> {
        if let Some(c) = count {
            sqlx::query("UPDATE shopitems SET count = ? WHERE id = ?")
                .bind(c)
                .bind(item_id)
                .execute(pool)
                .await?;
        }

        if let Some(p) = price {
            sqlx::query("UPDATE shopitems SET price = ? WHERE id = ?")
                .bind(p)
                .bind(item_id)
                .execute(pool)
                .await?;
        }

        if let Some(l) = location {
            sqlx::query("UPDATE shopitems SET location = ? WHERE id = ?")
                .bind(l)
                .bind(item_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    // 删除商品
    pub async fn delete_item(pool: &SqlitePool, item_id: i64) -> DbResult<()> {
        sqlx::query("DELETE FROM shopitems WHERE id = ?")
            .bind(item_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    // 获取所有交易记录
    pub async fn get_all_transactions(pool: &SqlitePool) -> DbResult<Vec<ShopLog>> {
        let logs = sqlx::query_as::<_, ShopLog>(
            "SELECT id, buyer, count, price, name, time, seller, location
             FROM shoplog ORDER BY time DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    // 获取用户购买记录
    pub async fn get_user_purchases(pool: &SqlitePool, buyer: &str) -> DbResult<Vec<ShopLog>> {
        let logs = sqlx::query_as::<_, ShopLog>(
            "SELECT id, buyer, count, price, name, time, seller, location
             FROM shoplog WHERE buyer = ? ORDER BY time DESC",
        )
        .bind(buyer)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    // 获取用户销售记录
    pub async fn get_user_sales(pool: &SqlitePool, seller: &str) -> DbResult<Vec<ShopLog>> {
        let logs = sqlx::query_as::<_, ShopLog>(
            "SELECT id, buyer, count, price, name, time, seller, location
             FROM shoplog WHERE seller = ? ORDER BY time DESC",
        )
        .bind(seller)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }

    // 搜索商品
    pub async fn search_items(pool: &SqlitePool, keyword: &str) -> DbResult<Vec<ShopItem>> {
        let pattern = format!("%{}%", keyword);
        let items = sqlx::query_as::<_, ShopItem>(
            "SELECT id, count, price, name, seller, location
             FROM shopitems WHERE name LIKE ? AND count > 0",
        )
        .bind(pattern)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }
}
