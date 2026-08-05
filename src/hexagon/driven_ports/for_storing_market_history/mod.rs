//! Conversation required to persist market-price history.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::market_history::MarketHistory};

/// Required interface for storing prices, dividends, and splits.
#[async_trait]
pub trait ForStoringMarketHistory: Send + Sync {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64>;
}
