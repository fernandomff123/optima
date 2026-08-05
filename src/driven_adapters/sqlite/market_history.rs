use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::error::Error;

use crate::hexagon::domain::market_history::{DailyQuote, Dividend, MarketHistory, StockSplit};
use crate::hexagon::{
    PortError, PortResult,
    driven_ports::{
        for_loading_market_history::ForLoadingMarketHistory,
        for_storing_market_history::ForStoringMarketHistory,
    },
};

#[derive(Clone)]
pub struct SqliteMarketHistoryAdapter {
    pool: SqlitePool,
}

impl SqliteMarketHistoryAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingMarketHistory for SqliteMarketHistoryAdapter {
    async fn load_market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        load_history(&self.pool, ticker)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}

#[async_trait::async_trait]
impl ForStoringMarketHistory for SqliteMarketHistoryAdapter {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64> {
        let report = insert_incremental(&self.pool, history)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))?;
        Ok(report.prices_affected + report.dividends_affected + report.splits_affected)
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct MarketHistoryStoreReport {
    pub prices_affected: u64,
    pub dividends_affected: u64,
    pub splits_affected: u64,
}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for (old, new) in [
        ("yahoo_prices", "market_prices"),
        ("yahoo_dividends", "dividends"),
        ("yahoo_splits", "stock_splits"),
    ] {
        rename_table_if_needed(pool, old, new).await?;
    }
    for index in [
        "idx_yahoo_prices_timestamp",
        "idx_yahoo_dividends_timestamp",
        "idx_yahoo_splits_timestamp",
    ] {
        sqlx::query(&format!("DROP INDEX IF EXISTS {index}"))
            .execute(pool)
            .await?;
    }
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS market_prices (
            ticker TEXT NOT NULL,
            timestamp TIMESTAMP NOT NULL,
            open REAL,
            high REAL,
            low REAL,
            close REAL,
            adjusted_close REAL,
            volume INTEGER,
            PRIMARY KEY (ticker, timestamp)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS dividends (
            ticker TEXT NOT NULL,
            timestamp TIMESTAMP NOT NULL,
            amount REAL NOT NULL,
            PRIMARY KEY (ticker, timestamp)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stock_splits (
            ticker TEXT NOT NULL,
            timestamp TIMESTAMP NOT NULL,
            numerator REAL NOT NULL,
            denominator REAL NOT NULL,
            ratio TEXT NOT NULL,
            PRIMARY KEY (ticker, timestamp)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_market_prices_timestamp ON market_prices (timestamp)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dividends_timestamp
         ON dividends (timestamp)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_stock_splits_timestamp ON stock_splits (timestamp)",
    )
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

pub async fn insert_incremental(
    pool: &SqlitePool,
    history: &MarketHistory,
) -> Result<MarketHistoryStoreReport, Box<dyn Error + Send + Sync>> {
    store_history(pool, history, false).await
}

pub async fn refresh_history(
    pool: &SqlitePool,
    history: &MarketHistory,
) -> Result<MarketHistoryStoreReport, Box<dyn Error + Send + Sync>> {
    store_history(pool, history, true).await
}

async fn store_history(
    pool: &SqlitePool,
    history: &MarketHistory,
    refresh_prices: bool,
) -> Result<MarketHistoryStoreReport, Box<dyn Error + Send + Sync>> {
    let mut transaction = pool.begin().await?;
    let mut report = MarketHistoryStoreReport::default();
    let price_sql = if refresh_prices {
        "INSERT INTO market_prices
         (ticker, timestamp, open, high, low, close, adjusted_close, volume)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (ticker, timestamp) DO UPDATE SET
            open = excluded.open,
            high = excluded.high,
            low = excluded.low,
            close = excluded.close,
            adjusted_close = excluded.adjusted_close,
            volume = excluded.volume"
    } else {
        "INSERT INTO market_prices
         (ticker, timestamp, open, high, low, close, adjusted_close, volume)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (ticker, timestamp) DO NOTHING"
    };

    for quote in &history.daily_quotes {
        let volume = quote.volume.map(i64::try_from).transpose()?;
        let result = sqlx::query(price_sql)
            .bind(&history.ticker)
            .bind(quote.timestamp)
            .bind(quote.open)
            .bind(quote.high)
            .bind(quote.low)
            .bind(quote.close)
            .bind(quote.adjusted_close)
            .bind(volume)
            .execute(&mut *transaction)
            .await?;
        report.prices_affected += result.rows_affected();
    }

    for dividend in &history.dividends {
        let result = sqlx::query(
            "INSERT INTO dividends (ticker, timestamp, amount)
             VALUES (?, ?, ?)
             ON CONFLICT (ticker, timestamp) DO UPDATE SET amount = excluded.amount",
        )
        .bind(&history.ticker)
        .bind(dividend.timestamp)
        .bind(dividend.amount)
        .execute(&mut *transaction)
        .await?;
        report.dividends_affected += result.rows_affected();
    }

    for split in &history.splits {
        let result = sqlx::query(
            "INSERT INTO stock_splits
             (ticker, timestamp, numerator, denominator, ratio)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (ticker, timestamp) DO UPDATE SET
                numerator = excluded.numerator,
                denominator = excluded.denominator,
                ratio = excluded.ratio",
        )
        .bind(&history.ticker)
        .bind(split.timestamp)
        .bind(split.numerator)
        .bind(split.denominator)
        .bind(&split.ratio)
        .execute(&mut *transaction)
        .await?;
        report.splits_affected += result.rows_affected();
    }

    transaction.commit().await?;
    Ok(report)
}

pub async fn latest_timestamp(
    pool: &SqlitePool,
    ticker: &str,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar("SELECT MAX(timestamp) FROM market_prices WHERE ticker = ?")
        .bind(ticker.trim().to_ascii_uppercase())
        .fetch_one(pool)
        .await
}

pub async fn contains_new_events(
    pool: &SqlitePool,
    history: &MarketHistory,
) -> Result<bool, sqlx::Error> {
    for dividend in &history.dividends {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM dividends WHERE ticker = ? AND timestamp = ?
             )",
        )
        .bind(&history.ticker)
        .bind(dividend.timestamp)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Ok(true);
        }
    }

    for split in &history.splits {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM stock_splits WHERE ticker = ? AND timestamp = ?
             )",
        )
        .bind(&history.ticker)
        .bind(split.timestamp)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn load_history(
    pool: &SqlitePool,
    ticker: &str,
) -> Result<MarketHistory, Box<dyn Error + Send + Sync>> {
    let ticker = ticker.trim().to_ascii_uppercase();
    let price_rows = sqlx::query(
        "SELECT timestamp, open, high, low, close, adjusted_close, volume
         FROM market_prices WHERE ticker = ? ORDER BY timestamp",
    )
    .bind(&ticker)
    .fetch_all(pool)
    .await?;
    let dividend_rows = sqlx::query(
        "SELECT timestamp, amount FROM dividends
         WHERE ticker = ? ORDER BY timestamp",
    )
    .bind(&ticker)
    .fetch_all(pool)
    .await?;
    let split_rows = sqlx::query(
        "SELECT timestamp, numerator, denominator, ratio FROM stock_splits
         WHERE ticker = ? ORDER BY timestamp",
    )
    .bind(&ticker)
    .fetch_all(pool)
    .await?;

    let daily_quotes = price_rows
        .into_iter()
        .map(|row| {
            let volume: Option<i64> = row.try_get("volume")?;
            Ok(DailyQuote {
                timestamp: row.try_get("timestamp")?,
                open: row.try_get("open")?,
                high: row.try_get("high")?,
                low: row.try_get("low")?,
                close: row.try_get("close")?,
                adjusted_close: row.try_get("adjusted_close")?,
                volume: volume.map(u64::try_from).transpose()?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error + Send + Sync>>>()?;
    let dividends = dividend_rows
        .into_iter()
        .map(|row| {
            Ok(Dividend {
                timestamp: row.try_get("timestamp")?,
                amount: row.try_get("amount")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let splits = split_rows
        .into_iter()
        .map(|row| {
            Ok(StockSplit {
                timestamp: row.try_get("timestamp")?,
                numerator: row.try_get("numerator")?,
                denominator: row.try_get("denominator")?,
                ratio: row.try_get("ratio")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok(MarketHistory {
        ticker,
        currency: None,
        exchange_timezone: None,
        daily_quotes,
        dividends,
        splits,
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn sample_history() -> MarketHistory {
        MarketHistory {
            ticker: "AAPL".to_string(),
            currency: Some("USD".to_string()),
            exchange_timezone: Some("America/New_York".to_string()),
            daily_quotes: vec![DailyQuote {
                timestamp: Utc.with_ymd_and_hms(2025, 1, 2, 14, 30, 0).unwrap(),
                open: Some(242.0),
                high: Some(244.0),
                low: Some(241.0),
                close: Some(243.0),
                adjusted_close: Some(242.5),
                volume: Some(50_000_000),
            }],
            dividends: vec![Dividend {
                timestamp: Utc.with_ymd_and_hms(2025, 1, 3, 14, 30, 0).unwrap(),
                amount: 0.25,
            }],
            splits: vec![StockSplit {
                timestamp: Utc.with_ymd_and_hms(2025, 1, 4, 14, 30, 0).unwrap(),
                numerator: 4.0,
                denominator: 1.0,
                ratio: "4:1".to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn stores_prices_dividends_and_splits_separately() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let expected = sample_history();

        assert!(contains_new_events(&pool, &expected).await.unwrap());
        let report = insert_incremental(&pool, &expected).await.unwrap();
        assert_eq!(report.prices_affected, 1);
        assert_eq!(report.dividends_affected, 1);
        assert_eq!(report.splits_affected, 1);
        assert!(!contains_new_events(&pool, &expected).await.unwrap());

        let loaded = load_history(&pool, "aapl").await.unwrap();
        assert_eq!(loaded.daily_quotes, expected.daily_quotes);
        assert_eq!(loaded.dividends, expected.dividends);
        assert_eq!(loaded.splits, expected.splits);
    }

    #[tokio::test]
    async fn migrates_legacy_price_table_without_losing_rows() {
        let pool = memory_pool().await;
        sqlx::query(
            "CREATE TABLE yahoo_prices (
                ticker TEXT NOT NULL, timestamp TIMESTAMP NOT NULL,
                open REAL, high REAL, low REAL, close REAL,
                adjusted_close REAL, volume INTEGER,
                PRIMARY KEY (ticker, timestamp)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO yahoo_prices (ticker, timestamp, close)
             VALUES ('IBM', '2026-07-14T13:30:00Z', 282.9)",
        )
        .execute(&pool)
        .await
        .unwrap();

        initialize(&pool).await.unwrap();

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_prices")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 1);
    }
}
