use sqlx::SqlitePool;

use chrono::{DateTime, Utc};

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        options::Snapshot,
        volatility::{ConstantMaturityVolatilityPoint, TermStructure},
    },
    driven_ports::{
        for_loading_option_chain_archive::{ArchivedOptionChain, ForLoadingOptionChainArchive},
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_reference_prices::ForLoadingReferencePrices,
        for_loading_volatility_term_structure_archive::ForLoadingVolatilityTermStructureArchive,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
        for_storing_option_chains::ForStoringOptionChains,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
    },
};

use super::{market_history, option_snapshots, volatility_term_structures};

/// SQLite adapter for option-analysis inputs stored by the application.
#[derive(Clone)]
pub struct SqliteOptionDataAdapter {
    pool: SqlitePool,
}

#[async_trait::async_trait]
impl ForLoadingVolatilityTermStructureArchive for SqliteOptionDataAdapter {
    async fn load_volatility_term_structure_archive(&self) -> PortResult<Vec<TermStructure>> {
        volatility_term_structures::load_all_current(&self.pool)
            .await
            .map_err(unavailable)
    }
}

#[async_trait::async_trait]
impl ForLoadingOptionChainArchive for SqliteOptionDataAdapter {
    async fn load_option_chain_archive(&self) -> PortResult<Vec<ArchivedOptionChain>> {
        option_snapshots::load_all_with_metadata(&self.pool)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(|item| ArchivedOptionChain {
                        snapshot: item.snapshot,
                        market_close: item.market_close,
                    })
                    .collect()
            })
            .map_err(unavailable)
    }
}

impl SqliteOptionDataAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingOptionChains for SqliteOptionDataAdapter {
    async fn load_option_chain(&self, ticker: &str) -> PortResult<Option<Snapshot>> {
        option_snapshots::load_latest(&self.pool, ticker)
            .await
            .map_err(unavailable)
    }
}

#[async_trait::async_trait]
impl ForLoadingVolatilityTermStructures for SqliteOptionDataAdapter {
    async fn load_term_structure(&self, ticker: &str) -> PortResult<Option<TermStructure>> {
        volatility_term_structures::load_latest(&self.pool, ticker)
            .await
            .map_err(unavailable)
    }

    async fn load_term_structure_at_or_before(
        &self,
        ticker: &str,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>> {
        volatility_term_structures::load_latest_at_or_before(&self.pool, ticker, Some(instant))
            .await
            .map_err(unavailable)
    }

    async fn load_constant_maturity_volatility_history(
        &self,
        ticker: &str,
        target_days: f64,
    ) -> PortResult<Vec<ConstantMaturityVolatilityPoint>> {
        volatility_term_structures::load_constant_maturity_history(&self.pool, ticker, target_days)
            .await
            .map_err(unavailable)
    }
}

#[async_trait::async_trait]
impl ForStoringOptionChains for SqliteOptionDataAdapter {
    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        market_close: DateTime<Utc>,
    ) -> PortResult<u64> {
        option_snapshots::save_snapshot(&self.pool, snapshot, market_close)
            .await
            .map(u64::from)
            .map_err(unavailable)
    }
}

#[async_trait::async_trait]
impl ForStoringVolatilityTermStructures for SqliteOptionDataAdapter {
    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64> {
        volatility_term_structures::insert(&self.pool, term_structure)
            .await
            .map_err(unavailable)
    }
}

#[async_trait::async_trait]
impl ForLoadingReferencePrices for SqliteOptionDataAdapter {
    async fn load_reference_price(&self, ticker: &str) -> PortResult<Option<f64>> {
        let history = market_history::load_history(&self.pool, ticker)
            .await
            .map_err(unavailable)?;
        Ok(history
            .daily_quotes
            .iter()
            .rev()
            .find_map(|quote| quote.close))
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}
