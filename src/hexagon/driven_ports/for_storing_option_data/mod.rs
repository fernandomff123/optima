//! Conversation required to persist option information.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{
    PortResult,
    domain::{options::Snapshot, volatility::TermStructure},
};

/// Required interface for storing option chains and their derived analytics.
#[async_trait]
pub trait ForStoringOptionData: Send + Sync {
    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        market_close: DateTime<Utc>,
    ) -> PortResult<u64>;

    async fn store_term_structure(&self, term_structure: &TermStructure) -> PortResult<u64>;
}
