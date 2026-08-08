//! Conversation offered to an operator migrating tracked ticker configuration.
use crate::hexagon::PortResult;
use async_trait::async_trait;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackedTickerMigrationReport {
    pub source_rows: u64,
    pub target_rows: u64,
}
#[async_trait]
pub trait ForMigratingTrackedTickers: Send + Sync {
    async fn migrate_tracked_tickers(&self) -> PortResult<TrackedTickerMigrationReport>;
}
