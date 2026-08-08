//! Conversation required to read historical option chains during a migration.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{PortResult, domain::options::Snapshot};

/// Technology-neutral archived observation, including its market session.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchivedOptionChain {
    pub snapshot: Snapshot,
    pub market_close: Option<DateTime<Utc>>,
}

/// Required interface for exporting every option-chain observation.
///
/// This port exists only for offline storage migrations. Normal use cases read
/// option chains through `ForLoadingOptionChains`.
#[async_trait]
pub trait ForLoadingOptionChainArchive: Send + Sync {
    async fn load_option_chain_archive(&self) -> PortResult<Vec<ArchivedOptionChain>>;
}
