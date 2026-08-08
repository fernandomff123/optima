//! Conversation offered to an operator migrating market histories.

use async_trait::async_trait;

use crate::hexagon::{PortResult, driven_ports::for_counting_market_history::MarketHistoryCounts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketHistoryMigrationReport {
    pub histories: u64,
    pub source: MarketHistoryCounts,
    pub target: MarketHistoryCounts,
}

#[async_trait]
pub trait ForMigratingMarketHistory: Send + Sync {
    async fn migrate_market_history(&self) -> PortResult<MarketHistoryMigrationReport>;
}
