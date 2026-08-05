//! Conversation required to persist index history.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::index_history::IndexHistory};

/// Required interface for storing volatility-index observations.
#[async_trait]
pub trait ForStoringIndexHistory: Send + Sync {
    async fn store_index_history(&self, history: &IndexHistory) -> PortResult<u64>;
}
