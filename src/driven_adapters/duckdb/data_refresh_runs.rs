//! DuckDB persistence for operational market-data refresh executions.

use crate::hexagon::{
    PortError, PortResult,
    domain::data_refresh::{
        DataRefreshFailure, DataRefreshOrigin, DataRefreshRun, DataRefreshState,
    },
    driven_ports::{
        for_loading_data_refresh_runs::ForLoadingDataRefreshRuns,
        for_storing_data_refresh_runs::ForStoringDataRefreshRuns,
    },
};
use duckdb::{Connection, params};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DuckDbDataRefreshRunsAdapter {
    database_path: PathBuf,
}
impl DuckDbDataRefreshRunsAdapter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: path.into(),
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
impl ForStoringDataRefreshRuns for DuckDbDataRefreshRunsAdapter {
    async fn store_data_refresh_run(&self, run: &DataRefreshRun) -> PortResult<()> {
        let path = self.database_path.clone();
        let run = run.clone();
        run_blocking(move || store(&path, &run)).await
    }
}
#[async_trait::async_trait]
impl ForLoadingDataRefreshRuns for DuckDbDataRefreshRunsAdapter {
    async fn load_recent_data_refresh_runs(&self, limit: usize) -> PortResult<Vec<DataRefreshRun>> {
        let path = self.database_path.clone();
        run_blocking(move || load_recent(&path, limit)).await
    }
    async fn load_running_data_refresh_runs(&self) -> PortResult<Vec<DataRefreshRun>> {
        let path = self.database_path.clone();
        run_blocking(move || load_with_query(&path,
            "SELECT id, origin, state, started_at, finished_at, target_session, items_obtained, items_persisted, failure_count, next_attempt_at, summary FROM data_refresh_runs WHERE state = 'running' ORDER BY started_at DESC, id DESC", None)).await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
    "CREATE TABLE IF NOT EXISTS data_refresh_runs (
        id VARCHAR PRIMARY KEY, origin VARCHAR NOT NULL, state VARCHAR NOT NULL,
        started_at TIMESTAMPTZ NOT NULL, finished_at TIMESTAMPTZ, target_session DATE NOT NULL,
        items_obtained UBIGINT NOT NULL, items_persisted UBIGINT NOT NULL, failure_count UBIGINT NOT NULL,
        next_attempt_at TIMESTAMPTZ, summary VARCHAR NOT NULL);
     CREATE TABLE IF NOT EXISTS data_refresh_failures (
        run_id VARCHAR NOT NULL, sequence UINTEGER NOT NULL, ticker VARCHAR NOT NULL,
        operation VARCHAR NOT NULL, error VARCHAR NOT NULL, PRIMARY KEY (run_id, sequence));
     CREATE INDEX IF NOT EXISTS idx_data_refresh_runs_started ON data_refresh_runs (started_at);
     CREATE INDEX IF NOT EXISTS idx_data_refresh_failures_run ON data_refresh_failures (run_id);" )
}

fn store(
    path: &PathBuf,
    run: &DataRefreshRun,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let transaction = connection.transaction()?;
    transaction.execute("INSERT INTO data_refresh_runs VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT DO UPDATE SET state=excluded.state, finished_at=excluded.finished_at,
        items_obtained=excluded.items_obtained, items_persisted=excluded.items_persisted,
        failure_count=excluded.failure_count, next_attempt_at=excluded.next_attempt_at, summary=excluded.summary",
        params![run.id, origin(run.origin), state(run.state), run.started_at, run.finished_at, run.target_session,
            run.items_obtained, run.items_persisted, run.failure_count, run.next_attempt_at, run.summary])?;
    transaction.execute(
        "DELETE FROM data_refresh_failures WHERE run_id = ?",
        [&run.id],
    )?;
    for (sequence, failure) in run.failures.iter().enumerate() {
        transaction.execute(
            "INSERT INTO data_refresh_failures VALUES (?, ?, ?, ?, ?)",
            params![
                run.id,
                sequence as u32,
                failure.ticker,
                failure.operation,
                failure.error
            ],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn load_recent(
    path: &PathBuf,
    limit: usize,
) -> Result<Vec<DataRefreshRun>, Box<dyn std::error::Error + Send + Sync>> {
    load_with_query(path, "SELECT id, origin, state, started_at, finished_at, target_session,
        items_obtained, items_persisted, failure_count, next_attempt_at, summary FROM data_refresh_runs
        ORDER BY started_at DESC, id DESC LIMIT ?", Some(limit))
}

fn load_with_query(
    path: &PathBuf,
    query: &str,
    limit: Option<usize>,
) -> Result<Vec<DataRefreshRun>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let mut statement = connection.prepare(query)?;
    let mapper = |row: &duckdb::Row<'_>| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
        ))
    };
    let rows = match limit {
        Some(limit) => statement
            .query_map([limit as u64], mapper)?
            .collect::<Result<Vec<_>, _>>()?,
        None => statement
            .query_map([], mapper)?
            .collect::<Result<Vec<_>, _>>()?,
    };
    let mut result = Vec::with_capacity(rows.len());
    for (
        id,
        origin_value,
        state_value,
        started_at,
        finished_at,
        target_session,
        items_obtained,
        items_persisted,
        failure_count,
        next_attempt_at,
        summary,
    ) in rows
    {
        let mut failures_stmt=connection.prepare("SELECT ticker, operation, error FROM data_refresh_failures WHERE run_id=? ORDER BY sequence")?;
        let failures = failures_stmt
            .query_map([&id], |row| {
                Ok(DataRefreshFailure {
                    ticker: row.get(0)?,
                    operation: row.get(1)?,
                    error: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        result.push(DataRefreshRun {
            id,
            origin: parse_origin(&origin_value)?,
            state: parse_state(&state_value)?,
            started_at,
            finished_at,
            target_session,
            items_obtained,
            items_persisted,
            failure_count,
            next_attempt_at,
            summary,
            failures,
        });
    }
    Ok(result)
}
fn origin(value: DataRefreshOrigin) -> &'static str {
    match value {
        DataRefreshOrigin::Startup => "startup",
        DataRefreshOrigin::Scheduled => "scheduled",
        DataRefreshOrigin::Retry => "retry",
        DataRefreshOrigin::Manual => "manual",
    }
}
fn state(value: DataRefreshState) -> &'static str {
    match value {
        DataRefreshState::Running => "running",
        DataRefreshState::Completed => "completed",
        DataRefreshState::Partial => "partial",
        DataRefreshState::Failed => "failed",
    }
}
fn parse_origin(value: &str) -> Result<DataRefreshOrigin, duckdb::Error> {
    match value {
        "startup" => Ok(DataRefreshOrigin::Startup),
        "scheduled" => Ok(DataRefreshOrigin::Scheduled),
        "retry" => Ok(DataRefreshOrigin::Retry),
        "manual" => Ok(DataRefreshOrigin::Manual),
        _ => Err(duckdb::Error::InvalidParameterName(value.to_string())),
    }
}
fn parse_state(value: &str) -> Result<DataRefreshState, duckdb::Error> {
    match value {
        "running" => Ok(DataRefreshState::Running),
        "completed" => Ok(DataRefreshState::Completed),
        "partial" => Ok(DataRefreshState::Partial),
        "failed" => Ok(DataRefreshState::Failed),
        _ => Err(duckdb::Error::InvalidParameterName(value.to_string())),
    }
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
        .map_err(|e| PortError::Unavailable(e.to_string()))?
        .map_err(|e| PortError::Unavailable(e.to_string()))
}
