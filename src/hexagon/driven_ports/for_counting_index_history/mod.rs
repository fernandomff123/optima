//! Conversation required to count persisted index observations.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[async_trait]
pub trait ForCountingIndexHistory: Send + Sync {
    async fn count_index_history(&self) -> PortResult<u64>;
}
