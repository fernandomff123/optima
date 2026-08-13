//! Conversation required to load tracked market symbols.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::tracked_ticker::TrackedTicker};

#[async_trait]
pub trait ForLoadingTrackedTickers: Send + Sync {
    async fn load_tracked_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        self.load_active_tickers().await
    }

    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>>;
}
