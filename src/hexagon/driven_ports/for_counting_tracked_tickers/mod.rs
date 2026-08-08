//! Conversation required to count tracked ticker configurations.
use crate::hexagon::PortResult;
use async_trait::async_trait;
#[async_trait]
pub trait ForCountingTrackedTickers: Send + Sync {
    async fn count_tracked_tickers(&self) -> PortResult<u64>;
}
