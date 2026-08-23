//! Conversation offered to actors viewing interest-rate information.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{PortResult, domain::treasury::YieldCurve};

#[derive(Debug, Clone, PartialEq)]
pub struct InterestRateCurveProjection {
    pub date: NaiveDate,
    pub published_points: Vec<PublishedInterestRatePoint>,
    pub interpolated_points: Vec<InterpolatedInterestRatePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublishedInterestRatePoint {
    pub tenor: String,
    pub days: f64,
    pub rate_percent: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolatedInterestRatePoint {
    pub days: f64,
    pub rate_percent: f64,
}

/// Provided interface for interest-rate queries.
#[async_trait]
pub trait ForViewingInterestRates: Send + Sync {
    async fn yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>>;

    async fn interest_rate_curve(
        &self,
        on_or_before: NaiveDate,
    ) -> PortResult<Option<InterestRateCurveProjection>>;

    async fn continuously_compounded_rate(
        &self,
        on_or_before: NaiveDate,
        days_to_maturity: f64,
    ) -> PortResult<Option<f64>>;
}
