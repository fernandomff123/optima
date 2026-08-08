//! Conversation required to count portfolio aggregates and events.
use crate::hexagon::PortResult;
use async_trait::async_trait;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioCounts {
    pub portfolios: u64,
    pub events: u64,
}
#[async_trait]
pub trait ForCountingPortfolios: Send + Sync {
    async fn count_portfolios(&self) -> PortResult<PortfolioCounts>;
}
