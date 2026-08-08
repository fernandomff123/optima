//! DuckDB persistence for tracked ticker configuration.

use std::path::PathBuf;

use duckdb::{Connection, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::TrackedTicker,
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
            initialize_schema(&connection)?;
            seed_defaults(&connection)
        })
        .await
    }
}

#[async_trait::async_trait]
impl ForLoadingTrackedTickers for DuckDbTrackedTickersAdapter {
    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        let path = self.database_path.clone();
        run_blocking(move || load_active(&path)).await
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
            active BOOLEAN NOT NULL,
            historical_prices BOOLEAN NOT NULL,
            option_snapshots BOOLEAN NOT NULL
        );",
    )
}

fn seed_defaults(connection: &Connection) -> Result<(), duckdb::Error> {
    for ticker in ["AAPL", "IBM", "GOOGL", "MSFT", "JPM", "SPY", "SPX"] {
        connection.execute(
            "INSERT INTO tracked_tickers VALUES (?, true, true, true) ON CONFLICT DO NOTHING",
            [ticker],
        )?;
    }
    Ok(())
}

fn store(path: &PathBuf, ticker: &TrackedTicker) -> Result<(), duckdb::Error> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    connection.execute(
        "INSERT INTO tracked_tickers (ticker, active, historical_prices, option_snapshots)
         VALUES (?, ?, ?, ?)
         ON CONFLICT DO UPDATE SET active = excluded.active,
            historical_prices = excluded.historical_prices,
            option_snapshots = excluded.option_snapshots",
        params![
            ticker.ticker.trim().to_ascii_uppercase(),
            ticker.active,
            ticker.historical_prices,
            ticker.option_snapshots
        ],
    )?;
    Ok(())
}

fn load_active(path: &PathBuf) -> Result<Vec<TrackedTicker>, duckdb::Error> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let mut statement = connection.prepare(
        "SELECT ticker, active, historical_prices, option_snapshots
         FROM tracked_tickers WHERE active ORDER BY ticker",
    )?;
    statement
        .query_map([], |row| {
            Ok(TrackedTicker {
                ticker: row.get(0)?,
                active: row.get(1)?,
                historical_prices: row.get(2)?,
                option_snapshots: row.get(3)?,
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
