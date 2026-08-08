//! Conversation offered to an operator migrating portfolios.
use crate::hexagon::{PortResult, driven_ports::for_counting_portfolios::PortfolioCounts};
use async_trait::async_trait;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioMigrationReport {
    pub source: PortfolioCounts,
    pub target: PortfolioCounts,
}
#[async_trait]
pub trait ForMigratingPortfolios: Send + Sync {
    async fn migrate_portfolios(&self) -> PortResult<PortfolioMigrationReport>;
}
