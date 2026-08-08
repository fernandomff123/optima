//! Conversation offered to an operator migrating yield curves.

use crate::hexagon::PortResult;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YieldCurveMigrationReport {
    pub source_rows: u64,
    pub target_rows: u64,
}

#[async_trait]
pub trait ForMigratingYieldCurves: Send + Sync {
    async fn migrate_yield_curves(&self) -> PortResult<YieldCurveMigrationReport>;
}
