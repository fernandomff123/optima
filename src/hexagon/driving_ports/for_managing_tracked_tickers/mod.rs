//! Conversation offered to actors configuring tracked market symbols.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::tracked_ticker::TrackedTicker};

#[async_trait]
pub trait ForManagingTrackedTickers: Send + Sync {
    async fn list_active_tickers(&self) -> PortResult<Vec<TrackedTicker>>;

    async fn configure_ticker(&self, ticker: TrackedTicker) -> PortResult<()>;
}
