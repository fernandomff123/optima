//! Conversation required to load data used in option analysis.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::hexagon::{
    PortResult,
    domain::{
        options::Snapshot,
        treasury::YieldCurve,
        volatility::{ConstantMaturityVolatilityPoint, TermStructure},
    },
};

/// Required interface for stored option-analysis inputs.
#[async_trait]
pub trait ForLoadingOptionData: Send + Sync {
    async fn load_option_chain(&self, ticker: &str) -> PortResult<Option<Snapshot>>;

    async fn load_term_structure(&self, ticker: &str) -> PortResult<Option<TermStructure>>;

    async fn load_term_structure_at_or_before(
        &self,
        ticker: &str,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>>;

    async fn load_reference_price(&self, ticker: &str) -> PortResult<Option<f64>>;

    async fn load_yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>>;

    async fn load_constant_maturity_volatility_history(
        &self,
        ticker: &str,
        target_days: f64,
    ) -> PortResult<Vec<ConstantMaturityVolatilityPoint>>;
}
