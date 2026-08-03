//! Conversation required to load previously stored market history.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::market_history::MarketHistory};

#[async_trait]
pub trait ForLoadingMarketHistory: Send + Sync {
    async fn load_market_history(&self, ticker: &str) -> PortResult<MarketHistory>;
}
