//! Conversation required to persist option-chain snapshots.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{PortResult, domain::options::Snapshot};

/// Required interface for storing one normalized option-chain observation.
#[async_trait]
pub trait ForStoringOptionChains: Send + Sync {
    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        market_close: DateTime<Utc>,
    ) -> PortResult<u64>;
}
