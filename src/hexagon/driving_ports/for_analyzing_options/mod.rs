//! Conversation offered to actors analyzing options.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortResult,
    domain::{
        options::Snapshot,
        simulation::Greeks,
        volatility::TermStructure,
        volatility_surface::{VolatilitySkew, VolatilitySurface},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreeksRequest {
    pub ticker: String,
    pub occ_symbol: String,
}

/// Provided interface grouping the complete option-analysis conversation.
#[async_trait]
pub trait ForAnalyzingOptions: Send + Sync {
    async fn option_chain(&self, ticker: &str) -> PortResult<Snapshot>;

    async fn term_structure(&self, ticker: &str) -> PortResult<TermStructure>;

    async fn volatility_surface(&self, ticker: &str) -> PortResult<VolatilitySurface>;

    async fn volatility_skew(
        &self,
        ticker: &str,
        expiration: NaiveDate,
    ) -> PortResult<VolatilitySkew>;

    async fn greeks(&self, request: GreeksRequest) -> PortResult<Greeks>;
}
