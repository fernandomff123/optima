//! Conversation required to load portfolios.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::portfolio::Portfolio};

/// Required interface implemented by a portfolio source or test double.
#[async_trait]
pub trait ForLoadingPortfolios: Send + Sync {
    async fn load_portfolio(&self, id: &str) -> PortResult<Option<Portfolio>>;
}
