//! Conversation offered to actors resolving an exact underlying.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{PortResult, domain::tracked_ticker::UnderlyingMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnderlyingResolution {
    pub ticker: String,
    pub validated_at: DateTime<Utc>,
    pub metadata: UnderlyingMetadata,
}

#[async_trait]
pub trait ForResolvingUnderlyings: Send + Sync {
    async fn resolve_underlying(&self, ticker: &str) -> PortResult<UnderlyingResolution>;
}
