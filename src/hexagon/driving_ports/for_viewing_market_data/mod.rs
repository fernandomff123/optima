//! Conversation used to view historical market information.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::{index_history::IndexHistory, live_price::LivePrice, market_history::MarketHistory},
};

/// Provided interface for actors intending to view market data.
#[async_trait]
pub trait ForViewingMarketData: Send + Sync {
    async fn market_history(&self, ticker: &str) -> PortResult<MarketHistory>;

    async fn index_history(&self, ticker: &str) -> PortResult<IndexHistory>;

    async fn live_price(&self, ticker: &str) -> PortResult<LivePrice>;
}
