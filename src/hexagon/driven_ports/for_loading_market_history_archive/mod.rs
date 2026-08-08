//! Conversation required to export market histories during offline migration.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::market_history::MarketHistory};

#[async_trait]
pub trait ForLoadingMarketHistoryArchive: Send + Sync {
    async fn load_market_history_archive(&self) -> PortResult<Vec<MarketHistory>>;
}
