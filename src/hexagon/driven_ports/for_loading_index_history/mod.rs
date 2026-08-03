//! Conversation required to load previously stored index history.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::index_history::IndexHistory};

/// Required interface for persistence actors that provide index observations.
#[async_trait]
pub trait ForLoadingIndexHistory: Send + Sync {
    async fn load_index_history(&self, ticker: &str) -> PortResult<IndexHistory>;
}
