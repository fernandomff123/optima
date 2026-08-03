//! Conversation required to persist the application's market data.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{
    PortResult,
    domain::{
        index_history::IndexHistory, market_history::MarketHistory, options::Snapshot,
        treasury::YieldCurve, volatility::TermStructure,
    },
};

/// Required persistence conversation for all synchronized market information.
///
/// These operations belong together because they are offered by the same
/// logical actor: the application's market-data store. The port does not expose
/// tables, SQL row types, or database-specific result objects.
#[async_trait]
pub trait ForStoringMarketData: Send + Sync {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64>;

    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        market_close: DateTime<Utc>,
    ) -> PortResult<u64>;

    async fn store_volatility_index(&self, history: &IndexHistory) -> PortResult<u64>;

    async fn store_yield_curves(&self, curves: &[YieldCurve]) -> PortResult<u64>;

    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64>;
}
