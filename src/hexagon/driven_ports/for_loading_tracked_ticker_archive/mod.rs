//! Conversation required to export all tracked ticker configurations.
use crate::hexagon::{PortResult, domain::tracked_ticker::TrackedTicker};
use async_trait::async_trait;
#[async_trait]
pub trait ForLoadingTrackedTickerArchive: Send + Sync {
    async fn load_tracked_ticker_archive(&self) -> PortResult<Vec<TrackedTicker>>;
}
