use sqlx::SqlitePool;

use chrono::{DateTime, NaiveDate, Utc};

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        options::Snapshot,
        treasury::YieldCurve,
        volatility::{ConstantMaturityVolatilityPoint, TermStructure},
    },
    driven_ports::{
        for_loading_option_data::ForLoadingOptionData,
        for_storing_option_data::ForStoringOptionData,
    },
};

use super::{market_history, option_snapshots, volatility_term_structures, yield_curves};

/// SQLite adapter for option-analysis inputs stored by the application.
#[derive(Clone)]
pub struct SqliteOptionDataAdapter {
    pool: SqlitePool,
}

impl SqliteOptionDataAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingOptionData for SqliteOptionDataAdapter {
    async fn load_option_chain(&self, ticker: &str) -> PortResult<Option<Snapshot>> {
        option_snapshots::load_latest(&self.pool, ticker)
            .await
            .map_err(unavailable)
    }

    async fn load_term_structure(&self, ticker: &str) -> PortResult<Option<TermStructure>> {
        let Some(snapshot) = option_snapshots::load_latest(&self.pool, ticker)
            .await
            .map_err(unavailable)?
        else {
            return Ok(None);
        };
        volatility_term_structures::load(&self.pool, ticker, snapshot.timestamp_utc)
            .await
            .map_err(unavailable)
    }

    async fn load_term_structure_at_or_before(
        &self,
        ticker: &str,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>> {
        let Some(snapshot) =
            option_snapshots::load_latest_at_or_before(&self.pool, ticker, instant)
                .await
                .map_err(unavailable)?
        else {
            return Ok(None);
        };
        volatility_term_structures::load(&self.pool, ticker, snapshot.timestamp_utc)
            .await
            .map_err(unavailable)
    }

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

    async fn load_yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        yield_curves::load_latest_on_or_before(&self.pool, on_or_before)
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
impl ForStoringOptionData for SqliteOptionDataAdapter {
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

    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64> {
        volatility_term_structures::insert(&self.pool, term_structure)
            .await
            .map_err(unavailable)
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}
