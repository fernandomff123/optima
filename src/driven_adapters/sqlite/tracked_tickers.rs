use sqlx::{Row, SqlitePool};

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::TrackedTicker,
    driven_ports::{
        for_loading_tracked_ticker_archive::ForLoadingTrackedTickerArchive,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
};

#[derive(Clone)]
pub struct SqliteTrackedTickersAdapter {
    pool: SqlitePool,
}

#[async_trait::async_trait]
impl ForLoadingTrackedTickerArchive for SqliteTrackedTickersAdapter {
    async fn load_tracked_ticker_archive(&self) -> PortResult<Vec<TrackedTicker>> {
        load_all(&self.pool).await.map_err(unavailable)
    }
}

impl SqliteTrackedTickersAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingTrackedTickers for SqliteTrackedTickersAdapter {
    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        initialize(&self.pool).await.map_err(unavailable)?;
        load_active(&self.pool).await.map_err(unavailable)
    }
}

#[async_trait::async_trait]
impl ForStoringTrackedTickers for SqliteTrackedTickersAdapter {
    async fn store_tracked_ticker(&self, ticker: &TrackedTicker) -> PortResult<()> {
        initialize(&self.pool).await.map_err(unavailable)?;
        upsert(&self.pool, ticker).await.map_err(unavailable)
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tracked_tickers (
            ticker TEXT PRIMARY KEY NOT NULL,
            active INTEGER NOT NULL DEFAULT 1,
            yahoo_prices INTEGER NOT NULL DEFAULT 0,
            cboe_snapshot INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert(pool: &SqlitePool, ticker: &TrackedTicker) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tracked_tickers
         (ticker, active, yahoo_prices, cboe_snapshot)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (ticker) DO UPDATE SET
            active = excluded.active,
            yahoo_prices = excluded.yahoo_prices,
            cboe_snapshot = excluded.cboe_snapshot",
    )
    .bind(ticker.ticker.trim().to_ascii_uppercase())
    .bind(ticker.active)
    .bind(ticker.historical_prices)
    .bind(ticker.option_snapshots)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_active(pool: &SqlitePool) -> Result<Vec<TrackedTicker>, sqlx::Error> {
    load_with_filter(pool, true).await
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<TrackedTicker>, sqlx::Error> {
    load_with_filter(pool, false).await
}

async fn load_with_filter(
    pool: &SqlitePool,
    active_only: bool,
) -> Result<Vec<TrackedTicker>, sqlx::Error> {
    let predicate = if active_only { "WHERE active = 1" } else { "" };
    let rows = sqlx::query(&format!(
        "SELECT ticker, active, yahoo_prices, cboe_snapshot
             FROM tracked_tickers {predicate} ORDER BY ticker"
    ))
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(TrackedTicker {
                ticker: row.try_get("ticker")?,
                active: row.try_get("active")?,
                historical_prices: row.try_get("yahoo_prices")?,
                option_snapshots: row.try_get("cboe_snapshot")?,
            })
        })
        .collect()
}

pub async fn set_option_snapshots(
    pool: &SqlitePool,
    ticker: &str,
    enabled: bool,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE tracked_tickers SET cboe_snapshot = ? WHERE ticker = ?")
        .bind(enabled)
        .bind(ticker.trim().to_ascii_uppercase())
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn seed_defaults(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let defaults = [
        TrackedTicker {
            ticker: "AAPL".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
        TrackedTicker {
            ticker: "IBM".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
        TrackedTicker {
            ticker: "GOOGL".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
        TrackedTicker {
            ticker: "MSFT".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
        TrackedTicker {
            ticker: "JPM".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
        TrackedTicker {
            ticker: "SPY".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
        TrackedTicker {
            ticker: "SPX".to_string(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        },
    ];

    for ticker in defaults {
        sqlx::query(
            "INSERT INTO tracked_tickers
             (ticker, active, yahoo_prices, cboe_snapshot)
             VALUES (?, ?, ?, ?)
             ON CONFLICT (ticker) DO NOTHING",
        )
        .bind(ticker.ticker)
        .bind(ticker.active)
        .bind(ticker.historical_prices)
        .bind(ticker.option_snapshots)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn seeds_and_updates_tracked_tickers() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        initialize(&pool).await.unwrap();
        seed_defaults(&pool).await.unwrap();

        let mut tickers = load_active(&pool).await.unwrap();
        assert_eq!(tickers.len(), 7);
        let spy = tickers.iter().find(|item| item.ticker == "SPY").unwrap();
        assert!(spy.option_snapshots);
        let spx = tickers.iter().find(|item| item.ticker == "SPX").unwrap();
        assert!(spx.option_snapshots);
        assert!(spx.historical_prices);
        assert!(tickers.iter().all(|item| item.option_snapshots));

        let aapl = tickers
            .iter_mut()
            .find(|item| item.ticker == "AAPL")
            .unwrap();
        aapl.active = false;
        upsert(&pool, aapl).await.unwrap();

        let active = load_active(&pool).await.unwrap();
        assert_eq!(active.len(), 6);
        assert!(!active.iter().any(|item| item.ticker == "AAPL"));
    }
}
