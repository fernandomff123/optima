//! Conversation required to store tracked market-symbol configuration.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::tracked_ticker::TrackedTicker};

#[async_trait]
pub trait ForStoringTrackedTickers: Send + Sync {
    async fn store_tracked_ticker(&self, ticker: &TrackedTicker) -> PortResult<()>;
}
