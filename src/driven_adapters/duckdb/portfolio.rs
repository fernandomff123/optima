//! DuckDB persistence for portfolios and their event ledgers.

use std::path::PathBuf;

use duckdb::{Connection, OptionalExt, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::portfolio::{Currency, Portfolio, PortfolioEvent},
    driven_ports::{
        for_counting_portfolios::{ForCountingPortfolios, PortfolioCounts},
        for_loading_portfolios::ForLoadingPortfolios,
        for_storing_portfolios::ForStoringPortfolios,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbPortfolioAdapter {
    database_path: PathBuf,
}

impl DuckDbPortfolioAdapter {
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
impl ForLoadingPortfolios for DuckDbPortfolioAdapter {
    async fn load_portfolio(&self, id: &str) -> PortResult<Option<Portfolio>> {
        let path = self.database_path.clone();
        let id = id.to_string();
        run_blocking(move || load(&path, &id)).await
    }
}

#[async_trait::async_trait]
impl ForStoringPortfolios for DuckDbPortfolioAdapter {
    async fn store_portfolio(&self, portfolio: &Portfolio) -> PortResult<()> {
        let path = self.database_path.clone();
        let portfolio = portfolio.clone();
        run_blocking(move || save(&path, &portfolio)).await
    }
}

#[async_trait::async_trait]
impl ForCountingPortfolios for DuckDbPortfolioAdapter {
    async fn count_portfolios(&self) -> PortResult<PortfolioCounts> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)?;
            connection.query_row(
                "SELECT (SELECT COUNT(*) FROM portfolios), (SELECT COUNT(*) FROM portfolio_events)",
                [],
                |row| {
                    Ok(PortfolioCounts {
                        portfolios: row.get(0)?,
                        events: row.get(1)?,
                    })
                },
            )
        })
        .await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS portfolios (
            portfolio_id VARCHAR PRIMARY KEY, name VARCHAR NOT NULL, base_currency VARCHAR NOT NULL
        );
        CREATE TABLE IF NOT EXISTS portfolio_events (
            portfolio_id VARCHAR NOT NULL, event_id VARCHAR NOT NULL,
            event_type VARCHAR NOT NULL, occurred_at TIMESTAMPTZ NOT NULL,
            payload JSON NOT NULL, PRIMARY KEY (portfolio_id, event_id)
        );
        CREATE INDEX IF NOT EXISTS idx_portfolio_events_time ON portfolio_events (portfolio_id, occurred_at);",
    )
}

fn save(
    path: &PathBuf,
    portfolio: &Portfolio,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let transaction = connection.transaction()?;
    transaction.execute("INSERT INTO portfolios VALUES (?, ?, ?) ON CONFLICT DO UPDATE SET name = excluded.name, base_currency = excluded.base_currency", params![&portfolio.id, &portfolio.name, portfolio.base_currency.code()])?;
    let mut statement = transaction.prepare(
        "INSERT INTO portfolio_events VALUES (?, ?, ?, ?, ?::JSON) ON CONFLICT DO NOTHING",
    )?;
    for event in portfolio.events() {
        statement.execute(params![
            &portfolio.id,
            event.id(),
            event.kind_name(),
            event.occurred_at(),
            serde_json::to_string(event)?
        ])?;
    }
    drop(statement);
    transaction.commit()?;
    Ok(())
}

fn load(
    path: &PathBuf,
    id: &str,
) -> Result<Option<Portfolio>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let metadata = connection
        .query_row(
            "SELECT name, base_currency FROM portfolios WHERE portfolio_id = ?",
            [id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((name, currency)) = metadata else {
        return Ok(None);
    };
    let mut portfolio = Portfolio::new(id, name, Currency::new(&currency)?)?;
    let mut statement = connection.prepare("SELECT payload::VARCHAR FROM portfolio_events WHERE portfolio_id = ? ORDER BY occurred_at, event_id")?;
    let payloads = statement.query_map([id], |row| row.get::<_, String>(0))?;
    for payload in payloads {
        portfolio.record(serde_json::from_str::<PortfolioEvent>(&payload?)?)?;
    }
    Ok(Some(portfolio))
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
