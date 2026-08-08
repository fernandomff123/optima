//! Conversation required to load calculated volatility term structures.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{
    PortResult,
    domain::volatility::{ConstantMaturityVolatilityPoint, TermStructure},
};

#[async_trait]
pub trait ForLoadingVolatilityTermStructures: Send + Sync {
    async fn load_term_structure(&self, ticker: &str) -> PortResult<Option<TermStructure>>;

    async fn load_term_structure_at_or_before(
        &self,
        ticker: &str,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>>;

    async fn load_constant_maturity_volatility_history(
        &self,
        ticker: &str,
        target_days: f64,
    ) -> PortResult<Vec<ConstantMaturityVolatilityPoint>>;
}
