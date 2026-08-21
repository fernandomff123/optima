//! DuckDB persistence for risk-free yield curves.

use std::path::PathBuf;

use chrono::NaiveDate;
use duckdb::{Connection, OptionalExt, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::treasury::YieldCurve,
    driven_ports::{
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbYieldCurvesAdapter {
    database_path: PathBuf,
}

impl DuckDbYieldCurvesAdapter {
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
impl ForLoadingYieldCurves for DuckDbYieldCurvesAdapter {
    async fn load_yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        let path = self.database_path.clone();
        run_blocking(move || load_curve(&path, on_or_before)).await
    }
}

#[async_trait::async_trait]
impl ForStoringYieldCurves for DuckDbYieldCurvesAdapter {
    async fn store_yield_curves(&self, curves: &[YieldCurve]) -> PortResult<u64> {
        let path = self.database_path.clone();
        let curves = curves.to_vec();
        run_blocking(move || store_curves(&path, &curves)).await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS yield_curves (
            observed_on DATE PRIMARY KEY,
            m1 DOUBLE, m2 DOUBLE, m3 DOUBLE, m6 DOUBLE,
            y1 DOUBLE, y2 DOUBLE, y3 DOUBLE, y5 DOUBLE,
            y7 DOUBLE, y10 DOUBLE, y20 DOUBLE, y30 DOUBLE
        );",
    )
}

fn store_curves(
    path: &PathBuf,
    curves: &[YieldCurve],
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        "CREATE TEMP TABLE incoming_yield_curves AS
         SELECT * FROM yield_curves WHERE false;",
    )?;
    {
        let mut appender = transaction.appender("incoming_yield_curves")?;
        for curve in curves {
            appender.append_row(params![
                curve.date, curve.m1, curve.m2, curve.m3, curve.m6, curve.y1, curve.y2, curve.y3,
                curve.y5, curve.y7, curve.y10, curve.y20, curve.y30,
            ])?;
        }
        appender.flush()?;
    }
    let affected = transaction.execute(
        "INSERT INTO yield_curves SELECT * FROM incoming_yield_curves
         ON CONFLICT DO UPDATE SET
            m1 = COALESCE(yield_curves.m1, excluded.m1),
            m2 = COALESCE(yield_curves.m2, excluded.m2),
            m3 = COALESCE(yield_curves.m3, excluded.m3),
            m6 = COALESCE(yield_curves.m6, excluded.m6),
            y1 = COALESCE(yield_curves.y1, excluded.y1),
            y2 = COALESCE(yield_curves.y2, excluded.y2),
            y3 = COALESCE(yield_curves.y3, excluded.y3),
            y5 = COALESCE(yield_curves.y5, excluded.y5),
            y7 = COALESCE(yield_curves.y7, excluded.y7),
            y10 = COALESCE(yield_curves.y10, excluded.y10),
            y20 = COALESCE(yield_curves.y20, excluded.y20),
            y30 = COALESCE(yield_curves.y30, excluded.y30)",
        [],
    )? as u64;
    transaction.commit()?;
    Ok(affected)
}

fn load_curve(
    path: &PathBuf,
    target: NaiveDate,
) -> Result<Option<YieldCurve>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    connection
        .query_row(
            "SELECT observed_on, m1, m2, m3, m6, y1, y2, y3, y5, y7, y10, y20, y30
             FROM yield_curves WHERE observed_on <= ?
             ORDER BY observed_on DESC LIMIT 1",
            [target],
            |row| {
                Ok(YieldCurve {
                    date: row.get(0)?,
                    m1: row.get(1)?,
                    m2: row.get(2)?,
                    m3: row.get(3)?,
                    m6: row.get(4)?,
                    y1: row.get(5)?,
                    y2: row.get(6)?,
                    y3: row.get(7)?,
                    y5: row.get(8)?,
                    y7: row.get(9)?,
                    y10: row.get(10)?,
                    y20: row.get(11)?,
                    y30: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
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
