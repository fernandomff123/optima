//! Conversation required to load stored option-chain snapshots.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::options::StoredOptionChainSnapshot};

/// Required interface for retrieving the latest stored chain for an asset.
#[async_trait]
pub trait ForLoadingOptionChains: Send + Sync {
    async fn load_option_chain(
        &self,
        ticker: &str,
    ) -> PortResult<Option<StoredOptionChainSnapshot>>;
}
