//! Conversation required to count persisted volatility term-structure points.
use crate::hexagon::PortResult;
use async_trait::async_trait;

#[async_trait]
pub trait ForCountingVolatilityTermStructures: Send + Sync {
    async fn count_volatility_term_structure_points(&self) -> PortResult<u64>;
}
