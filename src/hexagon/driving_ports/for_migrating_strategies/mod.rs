//! Conversation offered to an operator migrating saved strategies.

use async_trait::async_trait;

use crate::hexagon::{PortResult, driven_ports::for_counting_strategies::StrategyCounts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrategyMigrationReport {
    pub source: StrategyCounts,
    pub target: StrategyCounts,
}

#[async_trait]
pub trait ForMigratingStrategies: Send + Sync {
    async fn migrate_strategies(&self) -> PortResult<StrategyMigrationReport>;
}
