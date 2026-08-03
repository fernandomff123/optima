//! Conversation required to obtain historical prices and corporate actions.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{PortResult, domain::market_history::MarketHistory};

/// Required interface implemented by a market-history provider or test double.
#[async_trait]
pub trait ForObtainingMarketHistory: Send + Sync {
    async fn obtain_market_history(
        &self,
        ticker: &str,
        since: NaiveDate,
    ) -> PortResult<MarketHistory>;
}
