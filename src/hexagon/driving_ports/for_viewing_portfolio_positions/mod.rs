//! Conversation offered to actors viewing valued portfolio positions.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::portfolio_valuation::ValuedPosition};

#[async_trait]
pub trait ForViewingPortfolioPositions: Send + Sync {
    async fn valued_positions(&self, portfolio_id: &str) -> PortResult<Vec<ValuedPosition>>;
}
