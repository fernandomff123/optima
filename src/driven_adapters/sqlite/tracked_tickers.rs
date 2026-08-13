use sqlx::{Row, SqlitePool};

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::{TrackedTicker, TrackedTickerSource},
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
    async fn load_tracked_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        initialize(&self.pool).await.map_err(unavailable)?;
        load_all(&self.pool).await.map_err(unavailable)
    }

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
            source TEXT NOT NULL DEFAULT 'user',
            active INTEGER NOT NULL DEFAULT 1,
            yahoo_prices INTEGER NOT NULL DEFAULT 0,
            cboe_snapshot INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await?;
    let columns = sqlx::query("PRAGMA table_info(tracked_tickers)")
        .fetch_all(pool)
        .await?;
    if !columns
        .iter()
        .any(|column| column.get::<String, _>("name") == "source")
    {
        sqlx::query("ALTER TABLE tracked_tickers ADD COLUMN source TEXT NOT NULL DEFAULT 'user'")
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn upsert(pool: &SqlitePool, ticker: &TrackedTicker) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tracked_tickers
         (ticker, source, active, yahoo_prices, cboe_snapshot)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT (ticker) DO UPDATE SET
            source = excluded.source,
            active = excluded.active,
            yahoo_prices = excluded.yahoo_prices,
            cboe_snapshot = excluded.cboe_snapshot",
    )
    .bind(ticker.ticker.trim().to_ascii_uppercase())
    .bind(match ticker.source {
        TrackedTickerSource::System => "system",
        TrackedTickerSource::User => "user",
    })
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
        "SELECT ticker, source, active, yahoo_prices, cboe_snapshot
             FROM tracked_tickers {predicate} ORDER BY ticker"
    ))
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(TrackedTicker {
                ticker: row.try_get("ticker")?,
                source: match row.try_get::<String, _>("source")?.as_str() {
                    "system" => TrackedTickerSource::System,
                    _ => TrackedTickerSource::User,
                },
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

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn stores_and_updates_tracked_tickers() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        initialize(&pool).await.unwrap();
        upsert(
            &pool,
            &TrackedTicker {
                ticker: "AAPL".into(),
                source: TrackedTickerSource::User,
                active: true,
                historical_prices: true,
                option_snapshots: true,
            },
        )
        .await
        .unwrap();

        let mut tickers = load_active(&pool).await.unwrap();
        assert_eq!(tickers.len(), 1);

        let aapl = tickers
            .iter_mut()
            .find(|item| item.ticker == "AAPL")
            .unwrap();
        aapl.active = false;
        upsert(&pool, aapl).await.unwrap();

        let active = load_active(&pool).await.unwrap();
        assert!(active.is_empty());
        assert!(!active.iter().any(|item| item.ticker == "AAPL"));
    }
}
