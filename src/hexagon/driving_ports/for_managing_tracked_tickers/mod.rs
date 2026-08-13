//! Conversation offered to actors configuring tracked market symbols.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::tracked_ticker::{TrackedTicker, TrackedTickerConfiguration},
};

#[async_trait]
pub trait ForManagingTrackedTickers: Send + Sync {
    async fn list_tickers(&self, include_inactive: bool) -> PortResult<Vec<TrackedTicker>>;

    async fn bootstrap_system_tickers(&self) -> PortResult<()>;

    async fn configure_ticker(
        &self,
        ticker: &str,
        configuration: TrackedTickerConfiguration,
    ) -> PortResult<()>;
}
