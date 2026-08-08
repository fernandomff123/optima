//! Conversation required to count persisted option-chain observations.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionChainCounts {
    pub snapshots: u64,
    pub contracts: u64,
}

/// Required interface exposing technology-neutral option-chain counts.
#[async_trait]
pub trait ForCountingOptionChains: Send + Sync {
    async fn count_option_chains(&self) -> PortResult<OptionChainCounts>;
}
