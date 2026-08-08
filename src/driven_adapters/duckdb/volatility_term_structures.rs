//! DuckDB persistence for calculated volatility term-structure points.

use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use duckdb::{Connection, OptionalExt, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::volatility::{
        ConstantMaturityVolatilityPoint, TermStructure, TermStructurePoint, TermStructureSource,
    },
    driven_ports::{
        for_counting_volatility_term_structures::ForCountingVolatilityTermStructures,
        for_loading_reference_prices::ForLoadingReferencePrices,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
    },
};

const CALCULATION_VERSION: i32 = 3;

#[derive(Debug, Clone)]
pub struct DuckDbVolatilityTermStructuresAdapter {
    database_path: PathBuf,
}

impl DuckDbVolatilityTermStructuresAdapter {
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
impl ForLoadingVolatilityTermStructures for DuckDbVolatilityTermStructuresAdapter {
    async fn load_term_structure(&self, ticker: &str) -> PortResult<Option<TermStructure>> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || load_latest(&path, &ticker, None)).await
    }

    async fn load_term_structure_at_or_before(
        &self,
        ticker: &str,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || load_latest(&path, &ticker, Some(instant))).await
    }

    async fn load_constant_maturity_volatility_history(
        &self,
        ticker: &str,
        target_days: f64,
    ) -> PortResult<Vec<ConstantMaturityVolatilityPoint>> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || load_constant_history(&path, &ticker, target_days)).await
    }
}

#[async_trait::async_trait]
impl ForLoadingReferencePrices for DuckDbVolatilityTermStructuresAdapter {
    async fn load_reference_price(&self, ticker: &str) -> PortResult<Option<f64>> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            connection.query_row(
                "SELECT close FROM market_prices WHERE ticker = ? AND close IS NOT NULL ORDER BY observed_at DESC LIMIT 1",
                [&ticker], |row| row.get(0),
            ).optional()
        }).await
    }
}

#[async_trait::async_trait]
impl ForStoringVolatilityTermStructures for DuckDbVolatilityTermStructuresAdapter {
    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64> {
        let path = self.database_path.clone();
        let structure = term_structure.clone();
        run_blocking(move || store(&path, &structure)).await
    }
}

#[async_trait::async_trait]
impl ForCountingVolatilityTermStructures for DuckDbVolatilityTermStructuresAdapter {
    async fn count_volatility_term_structure_points(&self) -> PortResult<u64> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)?;
            connection.query_row(
                "SELECT COUNT(*) FROM volatility_term_structure_points",
                [],
                |row| row.get(0),
            )
        })
        .await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS volatility_term_structure_points (
            ticker VARCHAR NOT NULL, snapshot_timestamp TIMESTAMPTZ NOT NULL,
            treasury_date DATE NOT NULL, calculation_version INTEGER NOT NULL,
            days DOUBLE NOT NULL, variance DOUBLE NOT NULL, volatility DOUBLE NOT NULL,
            source_type VARCHAR NOT NULL, expiration DATE, interest_rate DOUBLE,
            near_expiration DATE, near_rate DOUBLE, next_expiration DATE, next_rate DOUBLE,
            PRIMARY KEY (ticker, snapshot_timestamp, calculation_version, days)
        );
        CREATE INDEX IF NOT EXISTS idx_volatility_terms_ticker_time
            ON volatility_term_structure_points (ticker, snapshot_timestamp);",
    )
}

fn store(
    path: &PathBuf,
    structure: &TermStructure,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let ticker = structure.ticker.trim().to_ascii_uppercase();
    let transaction = connection.transaction()?;
    transaction.execute_batch("CREATE TEMP TABLE incoming_volatility_terms AS SELECT * FROM volatility_term_structure_points WHERE false;")?;
    {
        let mut appender = transaction.appender("incoming_volatility_terms")?;
        for point in &structure.points {
            let (kind, expiration, rate, near_expiration, near_rate, next_expiration, next_rate) =
                match point.source {
                    TermStructureSource::Expiration {
                        expiration,
                        interest_rate,
                    } => (
                        "expiration",
                        Some(expiration),
                        Some(interest_rate),
                        None,
                        None,
                        None,
                        None,
                    ),
                    TermStructureSource::Interpolated {
                        near_expiration,
                        near_rate,
                        next_expiration,
                        next_rate,
                    } => (
                        "interpolated",
                        None,
                        None,
                        Some(near_expiration),
                        Some(near_rate),
                        Some(next_expiration),
                        Some(next_rate),
                    ),
                };
            appender.append_row(params![
                &ticker,
                structure.snapshot_timestamp,
                structure.treasury_date,
                CALCULATION_VERSION,
                point.days,
                point.variance,
                point.volatility,
                kind,
                expiration,
                rate,
                near_expiration,
                near_rate,
                next_expiration,
                next_rate
            ])?;
        }
        appender.flush()?;
    }
    let inserted = transaction.execute("INSERT INTO volatility_term_structure_points SELECT * FROM incoming_volatility_terms ON CONFLICT DO NOTHING", [])? as u64;
    transaction.commit()?;
    Ok(inserted)
}

fn load_latest(
    path: &PathBuf,
    ticker: &str,
    before: Option<DateTime<Utc>>,
) -> Result<Option<TermStructure>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let timestamp = match before {
        Some(instant) => connection.query_row("SELECT MAX(snapshot_timestamp) FROM volatility_term_structure_points WHERE ticker = ? AND calculation_version = ? AND snapshot_timestamp <= ?", params![ticker, CALCULATION_VERSION, instant], |row| row.get::<_, Option<DateTime<Utc>>>(0))?,
        None => connection.query_row("SELECT MAX(snapshot_timestamp) FROM volatility_term_structure_points WHERE ticker = ? AND calculation_version = ?", params![ticker, CALCULATION_VERSION], |row| row.get::<_, Option<DateTime<Utc>>>(0))?,
    };
    let Some(timestamp) = timestamp else {
        return Ok(None);
    };
    load_at(&connection, ticker, timestamp)
}

fn load_at(
    connection: &Connection,
    ticker: &str,
    timestamp: DateTime<Utc>,
) -> Result<Option<TermStructure>, Box<dyn std::error::Error + Send + Sync>> {
    let mut statement = connection.prepare("SELECT treasury_date, days, variance, volatility, source_type, expiration, interest_rate, near_expiration, near_rate, next_expiration, next_rate FROM volatility_term_structure_points WHERE ticker = ? AND snapshot_timestamp = ? AND calculation_version = ? ORDER BY days")?;
    let rows = statement
        .query_map(params![ticker, timestamp, CALCULATION_VERSION], |row| {
            let source_type: String = row.get(4)?;
            let source = if source_type == "expiration" {
                TermStructureSource::Expiration {
                    expiration: required(row, 5)?,
                    interest_rate: required(row, 6)?,
                }
            } else {
                TermStructureSource::Interpolated {
                    near_expiration: required(row, 7)?,
                    near_rate: required(row, 8)?,
                    next_expiration: required(row, 9)?,
                    next_rate: required(row, 10)?,
                }
            };
            Ok((
                row.get::<_, NaiveDate>(0)?,
                TermStructurePoint {
                    days: row.get(1)?,
                    variance: row.get(2)?,
                    volatility: row.get(3)?,
                    source,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let Some((treasury_date, _)) = rows.first() else {
        return Ok(None);
    };
    Ok(Some(TermStructure {
        ticker: ticker.to_string(),
        snapshot_timestamp: timestamp,
        treasury_date: *treasury_date,
        points: rows.into_iter().map(|(_, point)| point).collect(),
    }))
}

fn required<T: duckdb::types::FromSql>(
    row: &duckdb::Row<'_>,
    index: usize,
) -> Result<T, duckdb::Error> {
    row.get::<_, Option<T>>(index)?.ok_or_else(|| {
        duckdb::Error::InvalidColumnType(
            index,
            "required value".to_string(),
            duckdb::types::Type::Null,
        )
    })
}

fn load_constant_history(
    path: &PathBuf,
    ticker: &str,
    target_days: f64,
) -> Result<Vec<ConstantMaturityVolatilityPoint>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let mut statement = connection.prepare("SELECT snapshot_timestamp, volatility FROM volatility_term_structure_points WHERE ticker = ? AND calculation_version = ? AND ABS(days - ?) < 0.000001 ORDER BY snapshot_timestamp")?;
    let rows = statement.query_map(params![ticker, CALCULATION_VERSION, target_days], |row| {
        Ok((row.get::<_, DateTime<Utc>>(0)?, row.get::<_, f64>(1)?))
    })?;
    let mut by_session = BTreeMap::new();
    for row in rows {
        let (timestamp, volatility) = row?;
        by_session.insert(timestamp.date_naive(), volatility);
    }
    Ok(by_session
        .into_iter()
        .map(|(date, volatility)| ConstantMaturityVolatilityPoint {
            date,
            target_days,
            volatility,
        })
        .collect())
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
