//! SQLite persistence for risk-free yield curves.

use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::hexagon::domain::treasury::YieldCurve;

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS treasury (
            date DATE PRIMARY KEY NOT NULL,
            m1 REAL,
            m2 REAL,
            m3 REAL,
            m6 REAL,
            y1 REAL,
            y2 REAL,
            y3 REAL,
            y5 REAL,
            y7 REAL,
            y10 REAL,
            y20 REAL,
            y30 REAL
        )",
    )
    .execute(pool)
    .await?;

    add_column_if_missing(pool, "m2").await?;
    add_column_if_missing(pool, "y3").await?;
    add_column_if_missing(pool, "y7").await?;
    add_column_if_missing(pool, "y20").await?;

    Ok(())
}

async fn add_column_if_missing(pool: &SqlitePool, column: &str) -> Result<(), sqlx::Error> {
    let columns = sqlx::query("PRAGMA table_info(treasury)")
        .fetch_all(pool)
        .await?;
    if columns
        .iter()
        .any(|existing| existing.get::<String, _>("name") == column)
    {
        return Ok(());
    }
    let statement = format!("ALTER TABLE treasury ADD COLUMN {column} REAL");
    sqlx::query(&statement).execute(pool).await?;
    Ok(())
}

pub async fn insert_curves(pool: &SqlitePool, curves: &[YieldCurve]) -> Result<u64, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let mut inserted = 0;

    for curve in curves {
        let result = sqlx::query(
            "INSERT INTO treasury
             (date, m1, m2, m3, m6, y1, y2, y3, y5, y7, y10, y20, y30)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (date) DO UPDATE SET
                m2 = COALESCE(treasury.m2, excluded.m2),
                y3 = COALESCE(treasury.y3, excluded.y3),
                y7 = COALESCE(treasury.y7, excluded.y7),
                y20 = COALESCE(treasury.y20, excluded.y20)
             WHERE (treasury.m2 IS NULL AND excluded.m2 IS NOT NULL)
                OR (treasury.y3 IS NULL AND excluded.y3 IS NOT NULL)
                OR (treasury.y7 IS NULL AND excluded.y7 IS NOT NULL)
                OR (treasury.y20 IS NULL AND excluded.y20 IS NOT NULL)",
        )
        .bind(curve.date)
        .bind(curve.m1)
        .bind(curve.m2)
        .bind(curve.m3)
        .bind(curve.m6)
        .bind(curve.y1)
        .bind(curve.y2)
        .bind(curve.y3)
        .bind(curve.y5)
        .bind(curve.y7)
        .bind(curve.y10)
        .bind(curve.y20)
        .bind(curve.y30)
        .execute(&mut *transaction)
        .await?;
        inserted += result.rows_affected();
    }

    transaction.commit().await?;
    Ok(inserted)
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<YieldCurve>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT date, m1, m2, m3, m6, y1, y2, y3, y5, y7, y10, y20, y30
         FROM treasury
         ORDER BY date",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(YieldCurve {
                date: row.try_get("date")?,
                m1: row.try_get("m1")?,
                m2: row.try_get("m2")?,
                m3: row.try_get("m3")?,
                m6: row.try_get("m6")?,
                y1: row.try_get("y1")?,
                y2: row.try_get("y2")?,
                y3: row.try_get("y3")?,
                y5: row.try_get("y5")?,
                y7: row.try_get("y7")?,
                y10: row.try_get("y10")?,
                y20: row.try_get("y20")?,
                y30: row.try_get("y30")?,
            })
        })
        .collect()
}

pub async fn latest_date(pool: &SqlitePool) -> Result<Option<NaiveDate>, sqlx::Error> {
    sqlx::query_scalar("SELECT MAX(date) FROM treasury")
        .fetch_one(pool)
        .await
}

pub async fn load_latest_on_or_before(
    pool: &SqlitePool,
    target: NaiveDate,
) -> Result<Option<YieldCurve>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT date, m1, m2, m3, m6, y1, y2, y3, y5, y7, y10, y20, y30
         FROM treasury
         WHERE date <= ?
         ORDER BY date DESC
         LIMIT 1",
    )
    .bind(target)
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(YieldCurve {
            date: row.try_get("date")?,
            m1: row.try_get("m1")?,
            m2: row.try_get("m2")?,
            m3: row.try_get("m3")?,
            m6: row.try_get("m6")?,
            y1: row.try_get("y1")?,
            y2: row.try_get("y2")?,
            y3: row.try_get("y3")?,
            y5: row.try_get("y5")?,
            y7: row.try_get("y7")?,
            y10: row.try_get("y10")?,
            y20: row.try_get("y20")?,
            y30: row.try_get("y30")?,
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn sample_curve() -> YieldCurve {
        YieldCurve {
            date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            m1: Some(0.0445),
            m2: Some(0.0440),
            m3: Some(0.0436),
            m6: Some(0.0425),
            y1: Some(0.0417),
            y2: Some(0.0425),
            y3: Some(0.0430),
            y5: Some(0.0438),
            y7: Some(0.0448),
            y10: Some(0.0457),
            y20: Some(0.0470),
            y30: Some(0.0479),
        }
    }

    #[tokio::test]
    async fn inserts_and_loads_curves_idempotently() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let expected = vec![sample_curve()];

        assert_eq!(insert_curves(&pool, &expected).await.unwrap(), 1);
        assert_eq!(insert_curves(&pool, &expected).await.unwrap(), 0);
        assert_eq!(load_all(&pool).await.unwrap(), expected);
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente do Tesouro dos EUA"]
    async fn downloads_parses_and_stores_one_year() {
        let feed = crate::driven_adapters::treasury::download_ano("2025")
            .await
            .expect("o Tesouro deve devolver o ano de 2025");
        let curves = crate::driven_adapters::treasury::feed_to_yield_curves(feed)
            .expect("o XML real deve ser convertido para o domínio");

        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let inserted = insert_curves(&pool, &curves).await.unwrap();
        let loaded = load_all(&pool).await.unwrap();

        assert!(!curves.is_empty());
        assert_eq!(inserted as usize, curves.len());
        assert_eq!(loaded, curves);
    }
}
