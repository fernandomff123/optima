//! Conversation required to verify persisted strategy definitions.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrategyCounts {
    pub strategies: u64,
    pub legs: u64,
}

#[async_trait]
pub trait ForCountingStrategies: Send + Sync {
    async fn count_strategies(&self) -> PortResult<StrategyCounts>;
}
