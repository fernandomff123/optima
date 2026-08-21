//! DuckDB persistence for volatility-index histories.

use std::path::PathBuf;

use duckdb::{Connection, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::index_history::{DailyIndexPrice, IndexHistory},
    driven_ports::{
        for_loading_index_history::ForLoadingIndexHistory,
        for_storing_index_history::ForStoringIndexHistory,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbIndexHistoryAdapter {
    database_path: PathBuf,
}

impl DuckDbIndexHistoryAdapter {
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
impl ForLoadingIndexHistory for DuckDbIndexHistoryAdapter {
    async fn load_index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || load_history(&path, &ticker)).await
    }
}

#[async_trait::async_trait]
impl ForStoringIndexHistory for DuckDbIndexHistoryAdapter {
    async fn store_index_history(&self, history: &IndexHistory) -> PortResult<u64> {
        let path = self.database_path.clone();
        let history = history.clone();
        run_blocking(move || store_history(&path, &history)).await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS index_prices (
            ticker VARCHAR NOT NULL,
            observed_on DATE NOT NULL,
            open DOUBLE,
            high DOUBLE,
            low DOUBLE,
            close DOUBLE NOT NULL,
            PRIMARY KEY (ticker, observed_on)
        );
        CREATE INDEX IF NOT EXISTS idx_index_prices_ticker_date
            ON index_prices (ticker, observed_on);",
    )
}

fn store_history(
    path: &PathBuf,
    history: &IndexHistory,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let ticker = history.ticker.trim().to_ascii_uppercase();
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "CREATE TEMP TABLE incoming_index_prices AS
         SELECT * FROM index_prices WHERE false;",
    )?;
    {
        let mut appender = transaction.appender("incoming_index_prices")?;
        for price in &history.daily_prices {
            appender.append_row(params![
                &ticker,
                price.date,
                price.open,
                price.high,
                price.low,
                price.close,
            ])?;
        }
        appender.flush()?;
    }
    let inserted = transaction.execute(
        "INSERT INTO index_prices SELECT * FROM incoming_index_prices
         ON CONFLICT DO NOTHING",
        [],
    )? as u64;
    transaction.commit()?;
    Ok(inserted)
}

fn load_history(
    path: &PathBuf,
    ticker: &str,
) -> Result<IndexHistory, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let mut statement = connection.prepare(
        "SELECT observed_on, open, high, low, close FROM index_prices
         WHERE ticker = ? ORDER BY observed_on",
    )?;
    let daily_prices = statement
        .query_map([ticker], |row| {
            Ok(DailyIndexPrice {
                date: row.get(0)?,
                open: row.get(1)?,
                high: row.get(2)?,
                low: row.get(3)?,
                close: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IndexHistory {
        ticker: ticker.to_string(),
        daily_prices,
    })
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
