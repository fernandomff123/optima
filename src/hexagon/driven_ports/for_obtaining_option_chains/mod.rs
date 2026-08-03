//! Conversation required to obtain option-chain snapshots.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::options::Snapshot};

#[async_trait]
pub trait ForObtainingOptionChains: Send + Sync {
    async fn obtain_option_chain(&self, ticker: &str) -> PortResult<Snapshot>;
}
