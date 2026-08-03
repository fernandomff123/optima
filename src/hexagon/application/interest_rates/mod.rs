//! Interest-rate use cases.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortError, PortResult, domain::treasury::YieldCurve,
    driven_ports::for_loading_yield_curves::ForLoadingYieldCurves,
    driving_ports::for_viewing_interest_rates::ForViewingInterestRates,
};

/// Interest-rate application configured with its persistence participant.
pub struct InterestRatesApplication<YieldCurveStore> {
    yield_curve_store: YieldCurveStore,
}

impl<YieldCurveStore> InterestRatesApplication<YieldCurveStore> {
    pub fn new(yield_curve_store: YieldCurveStore) -> Self {
        Self { yield_curve_store }
    }
}

#[async_trait]
impl<YieldCurveStore> ForViewingInterestRates for InterestRatesApplication<YieldCurveStore>
where
    YieldCurveStore: ForLoadingYieldCurves,
{
    async fn yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        self.yield_curve_store.load_yield_curve(on_or_before).await
    }

    async fn continuously_compounded_rate(
        &self,
        on_or_before: NaiveDate,
        days_to_maturity: f64,
    ) -> PortResult<Option<f64>> {
        let Some(curve) = self
            .yield_curve_store
            .load_yield_curve(on_or_before)
            .await?
        else {
            return Ok(None);
        };
        crate::hexagon::domain::interest_rates::BoundedCubicSpline::from_treasury_curve(&curve)
            .and_then(|spline| spline.continuously_compounded_rate(days_to_maturity))
            .map(Some)
            .map_err(|error| PortError::InvalidRequest(error.to_string()))
    }
}
