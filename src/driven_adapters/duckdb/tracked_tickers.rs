//! DuckDB persistence for tracked ticker configuration.

use std::path::PathBuf;

use duckdb::{Connection, params, types::Type};

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::{
        TrackedTicker, TrackedTickerSource, UnderlyingMetadata, UnderlyingResolutionState,
    },
    driven_ports::{
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
        run_blocking(move || load(&path, LoadFilter::All)).await
    }

    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        let path = self.database_path.clone();
        run_blocking(move || load(&path, LoadFilter::Active)).await
    }

    async fn load_refresh_eligible_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        let path = self.database_path.clone();
        run_blocking(move || load(&path, LoadFilter::RefreshEligible)).await
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

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS tracked_tickers (
            ticker VARCHAR PRIMARY KEY,
            source VARCHAR NOT NULL DEFAULT 'user',
            active BOOLEAN NOT NULL,
            historical_prices BOOLEAN NOT NULL,
            option_snapshots BOOLEAN NOT NULL,
            resolution_state VARCHAR NOT NULL DEFAULT 'pending',
            validated_at TIMESTAMPTZ,
            currency VARCHAR,
            exchange VARCHAR,
            timezone VARCHAR,
            instrument_type VARCHAR
        );
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS source VARCHAR DEFAULT 'user';
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS resolution_state VARCHAR DEFAULT 'pending';
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS validated_at TIMESTAMPTZ;
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS currency VARCHAR;
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS exchange VARCHAR;
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS timezone VARCHAR;
        ALTER TABLE tracked_tickers ADD COLUMN IF NOT EXISTS instrument_type VARCHAR;
        UPDATE tracked_tickers SET source = 'user' WHERE source IS NULL;
        UPDATE tracked_tickers SET resolution_state = 'pending' WHERE resolution_state IS NULL;
        UPDATE tracked_tickers SET resolution_state = 'resolved' WHERE source = 'system';",
    )
}

fn store(path: &PathBuf, ticker: &TrackedTicker) -> Result<(), duckdb::Error> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    connection.execute(
        "INSERT INTO tracked_tickers (ticker, source, active, historical_prices, option_snapshots,
            resolution_state, validated_at, currency, exchange, timezone, instrument_type)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT DO UPDATE SET active = excluded.active,
            source = excluded.source,
            historical_prices = excluded.historical_prices,
            option_snapshots = excluded.option_snapshots,
            resolution_state = excluded.resolution_state,
            validated_at = excluded.validated_at,
            currency = excluded.currency,
            exchange = excluded.exchange,
            timezone = excluded.timezone,
            instrument_type = excluded.instrument_type",
        params![
            ticker.ticker.trim().to_ascii_uppercase(),
            match ticker.source {
                TrackedTickerSource::System => "system",
                TrackedTickerSource::User => "user",
            },
            ticker.active,
            ticker.historical_prices,
            ticker.option_snapshots,
            resolution_state(ticker.resolution_state),
            ticker.validated_at,
            ticker.metadata.currency,
            ticker.metadata.exchange,
            ticker.metadata.timezone,
            ticker.metadata.instrument_type,
        ],
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
enum LoadFilter {
    All,
    Active,
    RefreshEligible,
}

fn load(path: &PathBuf, filter: LoadFilter) -> Result<Vec<TrackedTicker>, duckdb::Error> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let predicate = match filter {
        LoadFilter::All => "",
        LoadFilter::Active => "WHERE active",
        LoadFilter::RefreshEligible => {
            "WHERE source = 'system' OR (active AND resolution_state = 'resolved')"
        }
    };
    let mut statement = connection.prepare(&format!(
        "SELECT ticker, source, active, historical_prices, option_snapshots, resolution_state,
            validated_at, currency, exchange, timezone, instrument_type
         FROM tracked_tickers {predicate} ORDER BY ticker"
    ))?;
    statement
        .query_map([], |row| {
            Ok(TrackedTicker {
                ticker: row.get(0)?,
                source: parse_source(&row.get::<_, String>(1)?)?,
                active: row.get(2)?,
                historical_prices: row.get(3)?,
                option_snapshots: row.get(4)?,
                resolution_state: parse_resolution_state(&row.get::<_, String>(5)?)?,
                validated_at: row.get(6)?,
                metadata: UnderlyingMetadata {
                    currency: row.get(7)?,
                    exchange: row.get(8)?,
                    timezone: row.get(9)?,
                    instrument_type: row.get(10)?,
                },
            })
        })?
        .collect()
}

fn resolution_state(value: UnderlyingResolutionState) -> &'static str {
    match value {
        UnderlyingResolutionState::Pending => "pending",
        UnderlyingResolutionState::Resolved => "resolved",
        UnderlyingResolutionState::Rejected => "rejected",
    }
}

fn parse_source(value: &str) -> Result<TrackedTickerSource, duckdb::Error> {
    match value {
        "system" => Ok(TrackedTickerSource::System),
        "user" => Ok(TrackedTickerSource::User),
        _ => Err(invalid_persisted_enum(1, "source", value)),
    }
}

fn parse_resolution_state(value: &str) -> Result<UnderlyingResolutionState, duckdb::Error> {
    match value {
        "pending" => Ok(UnderlyingResolutionState::Pending),
        "resolved" => Ok(UnderlyingResolutionState::Resolved),
        "rejected" => Ok(UnderlyingResolutionState::Rejected),
        _ => Err(invalid_persisted_enum(5, "resolution_state", value)),
    }
}

fn invalid_persisted_enum(index: usize, field: &str, value: &str) -> duckdb::Error {
    duckdb::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unknown tracked_tickers.{field} value: {value}"),
        )),
    )
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
