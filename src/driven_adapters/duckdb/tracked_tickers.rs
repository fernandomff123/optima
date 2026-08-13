//! DuckDB persistence for tracked ticker configuration.

use std::path::PathBuf;

use duckdb::{Connection, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::{TrackedTicker, TrackedTickerSource},
    driven_ports::{
        for_counting_tracked_tickers::ForCountingTrackedTickers,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbTrackedTickersAdapter {
    database_path: PathBuf,
}

impl DuckDbTrackedTickersAdapter {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
        }
    }

    pub async fn initialize(&self) -> PortResult<()> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)
        })
        .await
    }
}

#[async_trait::async_trait]
impl ForLoadingTrackedTickers for DuckDbTrackedTickersAdapter {
    async fn load_tracked_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        let path = self.database_path.clone();
        run_blocking(move || load(&path, false)).await
    }

    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        let path = self.database_path.clone();
        run_blocking(move || load(&path, true)).await
    }
}

#[async_trait::async_trait]
impl ForStoringTrackedTickers for DuckDbTrackedTickersAdapter {
    async fn store_tracked_ticker(&self, ticker: &TrackedTicker) -> PortResult<()> {
        let path = self.database_path.clone();
        let ticker = ticker.clone();
        run_blocking(move || store(&path, &ticker)).await
    }
}

#[async_trait::async_trait]
impl ForCountingTrackedTickers for DuckDbTrackedTickersAdapter {
    async fn count_tracked_tickers(&self) -> PortResult<u64> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)?;
            connection.query_row("SELECT COUNT(*) FROM tracked_tickers", [], |row| row.get(0))
        })
        .await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS tracked_tickers (
            ticker VARCHAR PRIMARY KEY,
            source VARCHAR NOT NULL DEFAULT 'user',
            active BOOLEAN NOT NULL,
            historical_prices BOOLEAN NOT NULL,
            option_snapshots BOOLEAN NOT NULL
        );
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS source VARCHAR DEFAULT 'user';
        UPDATE tracked_tickers SET source = 'user' WHERE source IS NULL;",
    )
}

fn store(path: &PathBuf, ticker: &TrackedTicker) -> Result<(), duckdb::Error> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    connection.execute(
        "INSERT INTO tracked_tickers (ticker, source, active, historical_prices, option_snapshots)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT DO UPDATE SET active = excluded.active,
            source = excluded.source,
            historical_prices = excluded.historical_prices,
            option_snapshots = excluded.option_snapshots",
        params![
            ticker.ticker.trim().to_ascii_uppercase(),
            match ticker.source {
                TrackedTickerSource::System => "system",
                TrackedTickerSource::User => "user",
            },
            ticker.active,
            ticker.historical_prices,
            ticker.option_snapshots
        ],
    )?;
    Ok(())
}

fn load(path: &PathBuf, active_only: bool) -> Result<Vec<TrackedTicker>, duckdb::Error> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let predicate = if active_only { "WHERE active" } else { "" };
    let mut statement = connection.prepare(&format!(
        "SELECT ticker, source, active, historical_prices, option_snapshots
         FROM tracked_tickers {predicate} ORDER BY ticker"
    ))?;
    statement
        .query_map([], |row| {
            Ok(TrackedTicker {
                ticker: row.get(0)?,
                source: match row.get::<_, String>(1)?.as_str() {
                    "system" => TrackedTickerSource::System,
                    _ => TrackedTickerSource::User,
                },
                active: row.get(2)?,
                historical_prices: row.get(3)?,
                option_snapshots: row.get(4)?,
            })
        })?
        .collect()
}

async fn run_blocking<T, E>(
    operation: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> PortResult<T>
where
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| PortError::Unavailable(error.to_string()))?
        .map_err(|error| PortError::Unavailable(error.to_string()))
}
