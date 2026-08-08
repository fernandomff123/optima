//! Conversation offered to an operator migrating option-chain storage.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionChainMigrationReport {
    pub source_snapshots: u64,
    pub source_contracts: u64,
    pub inserted_snapshots: u64,
    pub skipped_without_market_close: u64,
    pub target_snapshots: u64,
    pub target_contracts: u64,
}

/// Provided interface for the temporary offline migration conversation.
#[async_trait]
pub trait ForMigratingOptionChains: Send + Sync {
    async fn migrate_option_chains(&self) -> PortResult<OptionChainMigrationReport>;
}
