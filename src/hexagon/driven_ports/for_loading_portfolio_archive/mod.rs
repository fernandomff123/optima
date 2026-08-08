//! Conversation required to export every portfolio during migration.
use crate::hexagon::{PortResult, domain::portfolio::Portfolio};
use async_trait::async_trait;
#[async_trait]
pub trait ForLoadingPortfolioArchive: Send + Sync {
    async fn load_portfolio_archive(&self) -> PortResult<Vec<Portfolio>>;
}
