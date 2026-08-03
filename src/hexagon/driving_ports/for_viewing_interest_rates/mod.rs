//! Conversation offered to actors viewing interest-rate information.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{PortResult, domain::treasury::YieldCurve};

/// Provided interface for interest-rate queries.
#[async_trait]
pub trait ForViewingInterestRates: Send + Sync {
    async fn yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>>;

    async fn continuously_compounded_rate(
        &self,
        on_or_before: NaiveDate,
        days_to_maturity: f64,
    ) -> PortResult<Option<f64>>;
}
