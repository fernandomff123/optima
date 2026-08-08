//! Conversation required to persist calculated volatility term structures.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::volatility::TermStructure};

#[async_trait]
pub trait ForStoringVolatilityTermStructures: Send + Sync {
    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64>;
}
