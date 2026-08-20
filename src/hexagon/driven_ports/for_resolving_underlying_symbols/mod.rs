//! Conversation required to factually resolve an exact underlying symbol.

use async_trait::async_trait;

use crate::hexagon::domain::tracked_ticker::ResolvedUnderlying;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnderlyingResolutionError {
    NotFound(String),
    TemporarilyUnavailable(String),
    InvalidProviderResponse(String),
}

#[async_trait]
pub trait ForResolvingUnderlyingSymbols: Send + Sync {
    async fn resolve_underlying(
        &self,
        ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError>;
}
