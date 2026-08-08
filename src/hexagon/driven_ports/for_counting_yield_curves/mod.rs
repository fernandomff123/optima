//! Conversation required to count persisted yield curves.

use crate::hexagon::PortResult;
use async_trait::async_trait;

#[async_trait]
pub trait ForCountingYieldCurves: Send + Sync {
    async fn count_yield_curves(&self) -> PortResult<u64>;
}
