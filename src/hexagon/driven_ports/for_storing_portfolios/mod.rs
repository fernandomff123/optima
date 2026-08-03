//! Conversation required to store portfolios.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::portfolio::Portfolio};

#[async_trait]
pub trait ForStoringPortfolios: Send + Sync {
    async fn store_portfolio(&self, portfolio: &Portfolio) -> PortResult<()>;
}
