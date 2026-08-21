//! DuckDB persistence for prices, dividends, and stock splits.

use std::path::PathBuf;

use duckdb::{Connection, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::market_history::{DailyQuote, Dividend, MarketHistory, StockSplit},
    driven_ports::{
        for_loading_market_history::ForLoadingMarketHistory,
        for_storing_market_history::ForStoringMarketHistory,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbMarketHistoryAdapter {
    database_path: PathBuf,
}

impl DuckDbMarketHistoryAdapter {
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
impl ForLoadingMarketHistory for DuckDbMarketHistoryAdapter {
    async fn load_market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || load_history(&path, &ticker)).await
    }
}

#[async_trait::async_trait]
impl ForStoringMarketHistory for DuckDbMarketHistoryAdapter {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64> {
        let path = self.database_path.clone();
        let history = history.clone();
        run_blocking(move || store_history(&path, &history)).await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS market_prices (
            ticker VARCHAR NOT NULL,
            observed_at TIMESTAMPTZ NOT NULL,
            open DOUBLE,
            high DOUBLE,
            low DOUBLE,
            close DOUBLE,
            adjusted_close DOUBLE,
            volume UBIGINT,
            PRIMARY KEY (ticker, observed_at)
        );
        CREATE TABLE IF NOT EXISTS dividends (
            ticker VARCHAR NOT NULL,
            observed_at TIMESTAMPTZ NOT NULL,
            amount DOUBLE NOT NULL,
            PRIMARY KEY (ticker, observed_at)
        );
        CREATE TABLE IF NOT EXISTS stock_splits (
            ticker VARCHAR NOT NULL,
            observed_at TIMESTAMPTZ NOT NULL,
            numerator DOUBLE NOT NULL,
            denominator DOUBLE NOT NULL,
            ratio VARCHAR NOT NULL,
            PRIMARY KEY (ticker, observed_at)
        );
        CREATE INDEX IF NOT EXISTS idx_market_prices_ticker_time
            ON market_prices (ticker, observed_at);
        CREATE INDEX IF NOT EXISTS idx_dividends_ticker_time
            ON dividends (ticker, observed_at);
        CREATE INDEX IF NOT EXISTS idx_stock_splits_ticker_time
            ON stock_splits (ticker, observed_at);",
    )
}

fn store_history(
    path: &PathBuf,
    history: &MarketHistory,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let ticker = history.ticker.trim().to_ascii_uppercase();
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "CREATE TEMP TABLE incoming_market_prices AS
             SELECT * FROM market_prices WHERE false;
         CREATE TEMP TABLE incoming_dividends AS
             SELECT * FROM dividends WHERE false;
         CREATE TEMP TABLE incoming_stock_splits AS
             SELECT * FROM stock_splits WHERE false;",
    )?;
    {
        let mut appender = transaction.appender("incoming_market_prices")?;
        for quote in &history.daily_quotes {
            appender.append_row(params![
                &ticker,
                quote.timestamp,
                quote.open,
                quote.high,
                quote.low,
                quote.close,
                quote.adjusted_close,
                quote.volume,
            ])?;
        }
        appender.flush()?;
    }
    {
        let mut appender = transaction.appender("incoming_dividends")?;
        for dividend in &history.dividends {
            appender.append_row(params![&ticker, dividend.timestamp, dividend.amount])?;
        }
        appender.flush()?;
    }
    {
        let mut appender = transaction.appender("incoming_stock_splits")?;
        for split in &history.splits {
            appender.append_row(params![
                &ticker,
                split.timestamp,
                split.numerator,
                split.denominator,
                &split.ratio,
            ])?;
        }
        appender.flush()?;
    }
    let mut affected = transaction.execute(
        "INSERT INTO market_prices SELECT * FROM incoming_market_prices
         ON CONFLICT DO NOTHING",
        [],
    )? as u64;
    affected += transaction.execute(
        "INSERT INTO dividends SELECT * FROM incoming_dividends
         ON CONFLICT DO UPDATE SET amount = excluded.amount",
        [],
    )? as u64;
    affected += transaction.execute(
        "INSERT INTO stock_splits SELECT * FROM incoming_stock_splits
         ON CONFLICT DO UPDATE SET
            numerator = excluded.numerator,
            denominator = excluded.denominator,
            ratio = excluded.ratio",
        [],
    )? as u64;
    transaction.commit()?;
    Ok(affected)
}

fn load_history(
    path: &PathBuf,
    ticker: &str,
) -> Result<MarketHistory, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;

    let mut prices = connection.prepare(
        "SELECT observed_at, open, high, low, close, adjusted_close, volume
         FROM market_prices WHERE ticker = ? ORDER BY observed_at",
    )?;
    let daily_quotes = prices
        .query_map([ticker], |row| {
            Ok(DailyQuote {
                timestamp: row.get(0)?,
                open: row.get(1)?,
                high: row.get(2)?,
                low: row.get(3)?,
                close: row.get(4)?,
                adjusted_close: row.get(5)?,
                volume: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut dividend_statement = connection.prepare(
        "SELECT observed_at, amount FROM dividends
         WHERE ticker = ? ORDER BY observed_at",
    )?;
    let dividends = dividend_statement
        .query_map([ticker], |row| {
            Ok(Dividend {
                timestamp: row.get(0)?,
                amount: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut split_statement = connection.prepare(
        "SELECT observed_at, numerator, denominator, ratio FROM stock_splits
         WHERE ticker = ? ORDER BY observed_at",
    )?;
    let splits = split_statement
        .query_map([ticker], |row| {
            Ok(StockSplit {
                timestamp: row.get(0)?,
                numerator: row.get(1)?,
                denominator: row.get(2)?,
                ratio: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MarketHistory {
        ticker: ticker.to_string(),
        currency: None,
        exchange_timezone: None,
        daily_quotes,
        dividends,
        splits,
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
