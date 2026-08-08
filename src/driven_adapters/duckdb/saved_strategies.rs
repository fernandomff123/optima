//! DuckDB persistence for normalized saved strategy definitions.

use std::path::PathBuf;

use duckdb::{Connection, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::saved_strategy::{SavedStrategy, SavedStrategyLeg, StrategySide},
    driven_ports::{
        for_counting_strategies::{ForCountingStrategies, StrategyCounts},
        for_importing_strategy_archive::ForImportingStrategyArchive,
        for_loading_strategies::ForLoadingStrategies,
        for_storing_strategies::ForStoringStrategies,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbSavedStrategiesAdapter {
    database_path: PathBuf,
}

impl DuckDbSavedStrategiesAdapter {
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
impl ForLoadingStrategies for DuckDbSavedStrategiesAdapter {
    async fn load_strategies(&self) -> PortResult<Vec<SavedStrategy>> {
        let path = self.database_path.clone();
        run_blocking(move || load_all(&path)).await
    }
}

#[async_trait::async_trait]
impl ForStoringStrategies for DuckDbSavedStrategiesAdapter {
    async fn store_strategy(
        &self,
        name: &str,
        ticker: &str,
        legs: &[SavedStrategyLeg],
    ) -> PortResult<SavedStrategy> {
        let path = self.database_path.clone();
        let name = name.to_string();
        let ticker = ticker.to_string();
        let legs = legs.to_vec();
        run_blocking(move || store(&path, &name, &ticker, &legs)).await
    }

    async fn delete_strategy(&self, id: i64) -> PortResult<bool> {
        let path = self.database_path.clone();
        run_blocking(move || delete(&path, id)).await
    }
}

#[async_trait::async_trait]
impl ForImportingStrategyArchive for DuckDbSavedStrategiesAdapter {
    async fn import_strategy(&self, strategy: &SavedStrategy) -> PortResult<()> {
        let path = self.database_path.clone();
        let strategy = strategy.clone();
        run_blocking(move || import(&path, &strategy)).await
    }
}

#[async_trait::async_trait]
impl ForCountingStrategies for DuckDbSavedStrategiesAdapter {
    async fn count_strategies(&self) -> PortResult<StrategyCounts> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)?;
            connection.query_row(
                "SELECT (SELECT COUNT(*) FROM saved_strategies),
                        (SELECT COUNT(*) FROM saved_strategy_legs)",
                [],
                |row| {
                    Ok(StrategyCounts {
                        strategies: row.get(0)?,
                        legs: row.get(1)?,
                    })
                },
            )
        })
        .await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_strategies (
             strategy_id BIGINT PRIMARY KEY,
             name VARCHAR NOT NULL UNIQUE,
             ticker VARCHAR NOT NULL,
             updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
         );
         CREATE TABLE IF NOT EXISTS saved_strategy_legs (
             strategy_id BIGINT NOT NULL,
             leg_position INTEGER NOT NULL,
             occ_symbol VARCHAR NOT NULL,
             side VARCHAR NOT NULL,
             quantity UINTEGER NOT NULL,
             entry_price DOUBLE NOT NULL,
             PRIMARY KEY (strategy_id, leg_position)
         );
         CREATE INDEX IF NOT EXISTS idx_saved_strategies_ticker
             ON saved_strategies (ticker, name);",
    )
}

fn store(
    path: &PathBuf,
    name: &str,
    ticker: &str,
    legs: &[SavedStrategyLeg],
) -> Result<SavedStrategy, Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let transaction = connection.transaction()?;
    use duckdb::OptionalExt;

    let existing_id = transaction
        .query_row(
            "SELECT strategy_id FROM saved_strategies WHERE name = ?",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    let id = if let Some(id) = existing_id {
        transaction.execute(
            "UPDATE saved_strategies SET ticker = ?, updated_at = current_timestamp
             WHERE strategy_id = ?",
            params![ticker, id],
        )?;
        id
    } else {
        let id = transaction.query_row(
            "SELECT coalesce(max(strategy_id), 0) + 1 FROM saved_strategies",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        transaction.execute(
            "INSERT INTO saved_strategies (strategy_id, name, ticker) VALUES (?, ?, ?)",
            params![id, name, ticker],
        )?;
        id
    };
    replace_legs(&transaction, id, legs)?;
    transaction.commit()?;
    load_by_id(path, id)?.ok_or_else(|| "stored strategy could not be loaded".into())
}

fn import(
    path: &PathBuf,
    strategy: &SavedStrategy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO saved_strategies (strategy_id, name, ticker, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT (strategy_id) DO UPDATE SET name = excluded.name,
             ticker = excluded.ticker, updated_at = excluded.updated_at",
        params![
            strategy.id,
            &strategy.name,
            &strategy.ticker,
            strategy.updated_at
        ],
    )?;
    replace_legs(&transaction, strategy.id, &strategy.legs)?;
    transaction.commit()?;
    Ok(())
}

fn replace_legs(
    transaction: &duckdb::Transaction<'_>,
    strategy_id: i64,
    legs: &[SavedStrategyLeg],
) -> Result<(), duckdb::Error> {
    transaction.execute(
        "DELETE FROM saved_strategy_legs WHERE strategy_id = ?",
        [strategy_id],
    )?;
    let mut statement =
        transaction.prepare("INSERT INTO saved_strategy_legs VALUES (?, ?, ?, ?, ?, ?)")?;
    for (position, leg) in legs.iter().enumerate() {
        statement.execute(params![
            strategy_id,
            position as u32,
            &leg.occ_symbol,
            side_name(leg.side),
            leg.quantity,
            leg.entry_price
        ])?;
    }
    Ok(())
}

fn delete(path: &PathBuf, id: i64) -> Result<bool, duckdb::Error> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "DELETE FROM saved_strategy_legs WHERE strategy_id = ?",
        [id],
    )?;
    let deleted =
        transaction.execute("DELETE FROM saved_strategies WHERE strategy_id = ?", [id])?;
    transaction.commit()?;
    Ok(deleted == 1)
}

fn load_all(
    path: &PathBuf,
) -> Result<Vec<SavedStrategy>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let mut statement = connection.prepare(
        "SELECT strategy_id, name, ticker, updated_at
         FROM saved_strategies ORDER BY lower(name)",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get(3)?,
        ))
    })?;
    let metadata = rows.collect::<Result<Vec<_>, _>>()?;
    metadata
        .into_iter()
        .map(|(id, name, ticker, updated_at)| {
            Ok(SavedStrategy {
                id,
                name,
                ticker,
                legs: load_legs(&connection, id)?,
                updated_at,
            })
        })
        .collect()
}

fn load_by_id(
    path: &PathBuf,
    id: i64,
) -> Result<Option<SavedStrategy>, Box<dyn std::error::Error + Send + Sync>> {
    use duckdb::OptionalExt;

    let connection = Connection::open(path)?;
    let metadata = connection
        .query_row(
            "SELECT name, ticker, updated_at FROM saved_strategies WHERE strategy_id = ?",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((name, ticker, updated_at)) = metadata else {
        return Ok(None);
    };
    Ok(Some(SavedStrategy {
        id,
        name,
        ticker,
        legs: load_legs(&connection, id)?,
        updated_at,
    }))
}

fn load_legs(connection: &Connection, id: i64) -> Result<Vec<SavedStrategyLeg>, duckdb::Error> {
    let mut statement = connection.prepare(
        "SELECT occ_symbol, side, quantity, entry_price FROM saved_strategy_legs
         WHERE strategy_id = ? ORDER BY leg_position",
    )?;
    statement
        .query_map([id], |row| {
            let side = match row.get::<_, String>(1)?.as_str() {
                "BUY" => StrategySide::Buy,
                _ => StrategySide::Sell,
            };
            Ok(SavedStrategyLeg {
                occ_symbol: row.get(0)?,
                side,
                quantity: row.get(2)?,
                entry_price: row.get(3)?,
            })
        })?
        .collect()
}

fn side_name(side: StrategySide) -> &'static str {
    match side {
        StrategySide::Buy => "BUY",
        StrategySide::Sell => "SELL",
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
        .map_err(|error| PortError::Unavailable(error.to_string()))?
        .map_err(|error| PortError::Unavailable(error.to_string()))
}
