//! Conversation required to verify migrated market-history observations.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketHistoryCounts {
    pub prices: u64,
    pub dividends: u64,
    pub splits: u64,
}

#[async_trait]
pub trait ForCountingMarketHistory: Send + Sync {
    async fn count_market_history(&self) -> PortResult<MarketHistoryCounts>;
}
