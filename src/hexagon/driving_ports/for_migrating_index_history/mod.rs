//! Conversation offered to an operator migrating index histories.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexHistoryMigrationReport {
    pub indices: u64,
    pub source_rows: u64,
    pub target_rows: u64,
}

#[async_trait]
pub trait ForMigratingIndexHistory: Send + Sync {
    async fn migrate_index_history(&self) -> PortResult<IndexHistoryMigrationReport>;
}
