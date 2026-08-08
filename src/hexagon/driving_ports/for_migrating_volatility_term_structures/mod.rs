//! Conversation offered to an operator migrating volatility term structures.
use crate::hexagon::PortResult;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolatilityTermStructureMigrationReport {
    pub structures: u64,
    pub source_points: u64,
    pub target_points: u64,
}

#[async_trait]
pub trait ForMigratingVolatilityTermStructures: Send + Sync {
    async fn migrate_volatility_term_structures(
        &self,
    ) -> PortResult<VolatilityTermStructureMigrationReport>;
}
