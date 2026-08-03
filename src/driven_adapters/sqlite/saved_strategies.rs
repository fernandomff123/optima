//! SQLite adapter for saved strategy definitions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::hexagon::{
    PortError, PortResult,
    domain::saved_strategy::{SavedStrategy, SavedStrategyLeg},
    driven_ports::{
        for_loading_strategies::ForLoadingStrategies, for_storing_strategies::ForStoringStrategies,
    },
};

#[derive(Clone)]
pub struct SqliteSavedStrategiesAdapter {
    pool: SqlitePool,
}

impl SqliteSavedStrategiesAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ForLoadingStrategies for SqliteSavedStrategiesAdapter {
    async fn load_strategies(&self) -> PortResult<Vec<SavedStrategy>> {
        initialize(&self.pool).await.map_err(unavailable)?;
        let rows = sqlx::query(
            "SELECT id, name, ticker, legs, updated_at
             FROM saved_strategies ORDER BY name COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(unavailable)?;
        rows.into_iter().map(map_row).collect()
    }
}

#[async_trait]
impl ForStoringStrategies for SqliteSavedStrategiesAdapter {
    async fn store_strategy(
        &self,
        name: &str,
        ticker: &str,
        legs: &[SavedStrategyLeg],
    ) -> PortResult<SavedStrategy> {
        initialize(&self.pool).await.map_err(unavailable)?;
        let payload = rmp_serde::to_vec_named(legs).map_err(unavailable)?;
        sqlx::query(
            "INSERT INTO saved_strategies (name, ticker, legs)
             VALUES (?, ?, ?)
             ON CONFLICT (name) DO UPDATE SET
                 ticker = excluded.ticker,
                 legs = excluded.legs,
                 updated_at = CURRENT_TIMESTAMP",
        )
        .bind(name)
        .bind(ticker)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(unavailable)?;
        load_by_name(&self.pool, name)
            .await?
            .ok_or_else(|| PortError::Unavailable("stored strategy could not be loaded".into()))
    }

    async fn delete_strategy(&self, id: i64) -> PortResult<bool> {
        initialize(&self.pool).await.map_err(unavailable)?;
        sqlx::query("DELETE FROM saved_strategies WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() == 1)
            .map_err(unavailable)
    }
}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS saved_strategies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            ticker TEXT NOT NULL,
            legs BLOB NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_saved_strategies_ticker
         ON saved_strategies (ticker, name)",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn load_by_name(pool: &SqlitePool, name: &str) -> PortResult<Option<SavedStrategy>> {
    sqlx::query(
        "SELECT id, name, ticker, legs, updated_at
         FROM saved_strategies WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(unavailable)?
    .map(map_row)
    .transpose()
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> PortResult<SavedStrategy> {
    let payload: Vec<u8> = row.try_get("legs").map_err(unavailable)?;
    Ok(SavedStrategy {
        id: row.try_get("id").map_err(unavailable)?,
        name: row.try_get("name").map_err(unavailable)?,
        ticker: row.try_get("ticker").map_err(unavailable)?,
        legs: rmp_serde::from_slice(&payload).map_err(unavailable)?,
        updated_at: row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(unavailable)?,
    })
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}
