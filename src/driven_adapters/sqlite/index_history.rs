//! SQLite persistence for volatility-index histories.

use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::hexagon::domain::index_history::{DailyIndexPrice, IndexHistory};

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    rename_table_if_needed(pool, "cboe_index_prices", "index_prices").await?;
    sqlx::query("DROP INDEX IF EXISTS idx_cboe_index_prices_date")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS index_prices (
            ticker TEXT NOT NULL,
            date DATE NOT NULL,
            open REAL,
            high REAL,
            low REAL,
            close REAL NOT NULL,
            PRIMARY KEY (ticker, date)
        )",
    )
    .execute(pool)
    .await?;

    migrate_ohlc_to_nullable(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_index_prices_date
         ON index_prices (date)",
    )
    .execute(pool)
    .await?;

    sqlx::query("DROP TABLE IF EXISTS cboe_index_sync_sessions")
        .execute(pool)
        .await?;

    Ok(())
}

async fn rename_table_if_needed(
    pool: &SqlitePool,
    old: &str,
    new: &str,
) -> Result<(), sqlx::Error> {
    let old_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
    )
    .bind(old)
    .fetch_one(pool)
    .await?;
    let new_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
    )
    .bind(new)
    .fetch_one(pool)
    .await?;
    if old_exists && !new_exists {
        sqlx::query(&format!("ALTER TABLE {old} RENAME TO {new}"))
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn migrate_ohlc_to_nullable(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let columns = sqlx::query("PRAGMA table_info(index_prices)")
        .fetch_all(pool)
        .await?;
    let requires_migration = columns.iter().any(|column| {
        column.get::<String, _>("name") == "open" && column.get::<i64, _>("notnull") == 1
    });
    if !requires_migration {
        return Ok(());
    }

    let mut transaction = pool.begin().await?;
    sqlx::query("ALTER TABLE index_prices RENAME TO index_prices_legacy")
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "CREATE TABLE index_prices (
            ticker TEXT NOT NULL,
            date DATE NOT NULL,
            open REAL,
            high REAL,
            low REAL,
            close REAL NOT NULL,
            PRIMARY KEY (ticker, date)
        )",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO index_prices (ticker, date, open, high, low, close)
         SELECT ticker, date, open, high, low, close FROM index_prices_legacy",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query("DROP TABLE index_prices_legacy")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

    Ok(())
}

pub async fn insert_history(pool: &SqlitePool, history: &IndexHistory) -> Result<u64, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let mut inserted = 0;

    for price in &history.daily_prices {
        let result = sqlx::query(
            "INSERT INTO index_prices (ticker, date, open, high, low, close)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (ticker, date) DO NOTHING",
        )
        .bind(&history.ticker)
        .bind(price.date)
        .bind(price.open)
        .bind(price.high)
        .bind(price.low)
        .bind(price.close)
        .execute(&mut *transaction)
        .await?;
        inserted += result.rows_affected();
    }

    transaction.commit().await?;
    Ok(inserted)
}

pub async fn load_history(pool: &SqlitePool, ticker: &str) -> Result<IndexHistory, sqlx::Error> {
    let ticker = ticker.trim().to_ascii_uppercase();
    let rows = sqlx::query(
        "SELECT date, open, high, low, close
         FROM index_prices
         WHERE ticker = ?
         ORDER BY date",
    )
    .bind(&ticker)
    .fetch_all(pool)
    .await?;

    let daily_prices = rows
        .into_iter()
        .map(|row| {
            Ok(DailyIndexPrice {
                date: row.try_get("date")?,
                open: row.try_get("open")?,
                high: row.try_get("high")?,
                low: row.try_get("low")?,
                close: row.try_get("close")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok(IndexHistory {
        ticker,
        daily_prices,
    })
}

pub async fn load_all_histories(pool: &SqlitePool) -> Result<Vec<IndexHistory>, sqlx::Error> {
    let tickers =
        sqlx::query_scalar::<_, String>("SELECT DISTINCT ticker FROM index_prices ORDER BY ticker")
            .fetch_all(pool)
            .await?;
    let mut histories = Vec::with_capacity(tickers.len());
    for ticker in tickers {
        histories.push(load_history(pool, &ticker).await?);
    }
    Ok(histories)
}

pub async fn latest_date(
    pool: &SqlitePool,
    ticker: &str,
) -> Result<Option<NaiveDate>, sqlx::Error> {
    sqlx::query_scalar("SELECT MAX(date) FROM index_prices WHERE ticker = ?")
        .bind(ticker.trim().to_ascii_uppercase())
        .fetch_one(pool)
        .await
}

pub async fn load_latest_two_at_or_before(
    pool: &SqlitePool,
    ticker: &str,
    target: NaiveDate,
) -> Result<Vec<DailyIndexPrice>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT date, open, high, low, close
         FROM index_prices
         WHERE ticker = ? AND date <= ?
         ORDER BY date DESC
         LIMIT 2",
    )
    .bind(ticker.trim().to_ascii_uppercase())
    .bind(target)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(DailyIndexPrice {
                date: row.try_get("date")?,
                open: row.try_get("open")?,
                high: row.try_get("high")?,
                low: row.try_get("low")?,
                close: row.try_get("close")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn sample_history() -> IndexHistory {
        IndexHistory {
            ticker: "VIX".to_string(),
            daily_prices: vec![
                DailyIndexPrice {
                    date: NaiveDate::from_ymd_opt(1990, 1, 2).unwrap(),
                    open: Some(17.24),
                    high: Some(17.24),
                    low: Some(17.24),
                    close: 17.24,
                },
                DailyIndexPrice {
                    date: NaiveDate::from_ymd_opt(1990, 1, 3).unwrap(),
                    open: Some(18.19),
                    high: Some(18.19),
                    low: Some(18.19),
                    close: 18.19,
                },
            ],
        }
    }

    #[tokio::test]
    async fn inserts_and_loads_one_index_idempotently() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let expected = sample_history();

        assert_eq!(insert_history(&pool, &expected).await.unwrap(), 2);
        assert_eq!(insert_history(&pool, &expected).await.unwrap(), 0);

        let loaded = load_history(&pool, "vix").await.unwrap();
        assert_eq!(loaded, expected);
    }

    #[tokio::test]
    async fn migrates_legacy_index_table_without_losing_rows() {
        let pool = memory_pool().await;
        sqlx::query(
            "CREATE TABLE cboe_index_prices (
                ticker TEXT NOT NULL, date DATE NOT NULL, open REAL,
                high REAL, low REAL, close REAL NOT NULL,
                PRIMARY KEY (ticker, date)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO cboe_index_prices (ticker, date, close)
             VALUES ('VIX', '2026-07-14', 17.2)",
        )
        .execute(&pool)
        .await
        .unwrap();

        initialize(&pool).await.unwrap();

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_prices")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[tokio::test]
    async fn loads_the_latest_two_prices_at_or_before_a_date() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        insert_history(&pool, &sample_history()).await.unwrap();

        let prices = load_latest_two_at_or_before(
            &pool,
            "vix",
            NaiveDate::from_ymd_opt(1990, 1, 3).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].date, NaiveDate::from_ymd_opt(1990, 1, 3).unwrap());
        assert_eq!(prices[1].date, NaiveDate::from_ymd_opt(1990, 1, 2).unwrap());
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente da CBOE"]
    async fn downloads_parses_and_stores_vix() {
        let response = crate::driven_adapters::cboe::download_indice("VIX")
            .await
            .expect("a CBOE deve devolver o CSV do VIX");
        let history = crate::driven_adapters::cboe::response_to_index_history(response)
            .expect("o CSV do VIX deve ser convertido para o domínio");
        let expected_rows = history.daily_prices.len();

        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let inserted = insert_history(&pool, &history).await.unwrap();
        let loaded = load_history(&pool, "VIX").await.unwrap();

        assert!(expected_rows > 0);
        assert_eq!(inserted as usize, expected_rows);
        assert_eq!(loaded, history);
    }
}
