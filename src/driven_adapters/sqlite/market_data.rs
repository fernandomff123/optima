//! Aggregate SQLite adapter for the market-data persistence conversation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        index_history::IndexHistory, market_history::MarketHistory, options::Snapshot,
        treasury::YieldCurve, volatility::TermStructure,
    },
    driven_ports::for_storing_market_data::ForStoringMarketData,
};

#[derive(Clone)]
pub struct SqliteMarketDataAdapter {
    pool: SqlitePool,
}

impl SqliteMarketDataAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ForStoringMarketData for SqliteMarketDataAdapter {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64> {
        let report = super::market_history::insert_incremental(&self.pool, history)
            .await
            .map_err(unavailable)?;
        Ok(report.prices_affected + report.dividends_affected + report.splits_affected)
    }

    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        market_close: DateTime<Utc>,
    ) -> PortResult<u64> {
        super::option_snapshots::save_snapshot(&self.pool, snapshot, market_close)
            .await
            .map(u64::from)
            .map_err(unavailable)
    }

    async fn store_volatility_index(&self, history: &IndexHistory) -> PortResult<u64> {
        super::index_history::insert_history(&self.pool, history)
            .await
            .map_err(unavailable)
    }

    async fn store_yield_curves(&self, curves: &[YieldCurve]) -> PortResult<u64> {
        super::yield_curves::insert_curves(&self.pool, curves)
            .await
            .map_err(unavailable)
    }

    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64> {
        super::volatility_term_structures::insert(&self.pool, term_structure)
            .await
            .map_err(unavailable)
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}
