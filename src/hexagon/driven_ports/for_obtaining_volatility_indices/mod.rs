//! Conversation required to obtain volatility-index history.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::index_history::IndexHistory};

#[async_trait]
pub trait ForObtainingVolatilityIndices: Send + Sync {
    async fn obtain_volatility_index(&self, ticker: &str) -> PortResult<IndexHistory>;
}
