use crate::hexagon::domain::portfolio::{Currency, Portfolio, PortfolioEvent};
use crate::hexagon::{
    PortError, PortResult,
    driven_ports::{
        for_loading_portfolios::ForLoadingPortfolios, for_storing_portfolios::ForStoringPortfolios,
    },
};
use sqlx::{Row, SqlitePool};
use std::error::Error;

/// Driven adapter that stores portfolio event streams in SQLite.
#[derive(Clone)]
pub struct SqlitePortfolioAdapter {
    pool: SqlitePool,
}

impl SqlitePortfolioAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingPortfolios for SqlitePortfolioAdapter {
    async fn load_portfolio(&self, id: &str) -> PortResult<Option<Portfolio>> {
        load(&self.pool, id)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}

#[async_trait::async_trait]
impl ForStoringPortfolios for SqlitePortfolioAdapter {
    async fn store_portfolio(&self, portfolio: &Portfolio) -> PortResult<()> {
        save(&self.pool, portfolio)
            .await
            .map(|_| ())
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS portfolios (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            base_currency TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS portfolio_events (
            portfolio_id TEXT NOT NULL,
            event_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            occurred_at TIMESTAMP NOT NULL,
            payload BLOB NOT NULL,
            PRIMARY KEY (portfolio_id, event_id),
            FOREIGN KEY (portfolio_id) REFERENCES portfolios(id)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_portfolio_events_time
         ON portfolio_events (portfolio_id, occurred_at)",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn save(
    pool: &SqlitePool,
    portfolio: &Portfolio,
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    initialize(pool).await?;
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO portfolios (id, name, base_currency) VALUES (?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET name = excluded.name, base_currency = excluded.base_currency",
    )
    .bind(&portfolio.id)
    .bind(&portfolio.name)
    .bind(portfolio.base_currency.code())
    .execute(&mut *transaction)
    .await?;
    let mut inserted = 0;
    for event in portfolio.events() {
        let payload = rmp_serde::to_vec_named(event)?;
        inserted += sqlx::query(
            "INSERT INTO portfolio_events
             (portfolio_id, event_id, event_type, occurred_at, payload)
             VALUES (?, ?, ?, ?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(&portfolio.id)
        .bind(event.id())
        .bind(event.kind_name())
        .bind(event.occurred_at())
        .bind(payload)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    }
    transaction.commit().await?;
    Ok(inserted)
}

pub async fn load(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<Portfolio>, Box<dyn Error + Send + Sync>> {
    initialize(pool).await?;
    let Some(row) = sqlx::query("SELECT name, base_currency FROM portfolios WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let mut portfolio = Portfolio::new(
        id,
        row.try_get::<String, _>("name")?,
        Currency::new(&row.try_get::<String, _>("base_currency")?)?,
    )?;
    let payloads: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT payload FROM portfolio_events WHERE portfolio_id = ? ORDER BY occurred_at, event_id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    for payload in payloads {
        portfolio.record(rmp_serde::from_slice::<PortfolioEvent>(&payload)?)?;
    }
    Ok(Some(portfolio))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::domain::portfolio::{CashMovement, CashMovementKind, Money, decimal};
    use chrono::{TimeZone, Utc};
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn stores_and_reconstructs_the_event_ledger_idempotently() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let mut portfolio = Portfolio::new("main", "Principal", Currency::eur()).unwrap();
        portfolio
            .record(PortfolioEvent::CashMovement(CashMovement {
                id: "deposit-1".to_string(),
                occurred_at: Utc.with_ymd_and_hms(2026, 7, 17, 9, 0, 0).unwrap(),
                kind: CashMovementKind::Deposit,
                amount: Money::new(decimal("1000").unwrap(), Currency::eur()),
            }))
            .unwrap();

        assert_eq!(save(&pool, &portfolio).await.unwrap(), 1);
        assert_eq!(save(&pool, &portfolio).await.unwrap(), 0);
        let loaded = load(&pool, "main").await.unwrap().unwrap();
        assert_eq!(loaded, portfolio);
        assert_eq!(loaded.cash_balances()["EUR"], decimal("1000").unwrap());
    }
}
