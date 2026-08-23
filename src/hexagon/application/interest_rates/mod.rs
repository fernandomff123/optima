//! Interest-rate use cases.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortError, PortResult,
    domain::{interest_rates::BoundedCubicSpline, treasury::YieldCurve},
    driven_ports::for_loading_yield_curves::ForLoadingYieldCurves,
    driving_ports::for_viewing_interest_rates::{
        ForViewingInterestRates, InterestRateCurveProjection, InterpolatedInterestRatePoint,
        PublishedInterestRatePoint,
    },
};

const INTERPOLATED_MONTHS: u16 = 360;

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

    async fn interest_rate_curve(
        &self,
        on_or_before: NaiveDate,
    ) -> PortResult<Option<InterestRateCurveProjection>> {
        let Some(curve) = self
            .yield_curve_store
            .load_yield_curve(on_or_before)
            .await?
        else {
            return Ok(None);
        };
        let spline = BoundedCubicSpline::from_treasury_curve(&curve)
            .map_err(|error| PortError::Unavailable(error.to_string()))?;
        let interpolated_points = (1..=INTERPOLATED_MONTHS)
            .map(|month| {
                let days = f64::from(month) * 30.0;
                spline
                    .bond_equivalent_yield(days)
                    .map(|rate| InterpolatedInterestRatePoint {
                        days,
                        rate_percent: rate * 100.0,
                    })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PortError::Unavailable(error.to_string()))?;
        Ok(Some(InterestRateCurveProjection {
            date: curve.date,
            published_points: published_points(&curve),
            interpolated_points,
        }))
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
        BoundedCubicSpline::from_treasury_curve(&curve)
            .and_then(|spline| spline.continuously_compounded_rate(days_to_maturity))
            .map(Some)
            .map_err(|error| PortError::InvalidRequest(error.to_string()))
    }
}

fn published_points(curve: &YieldCurve) -> Vec<PublishedInterestRatePoint> {
    [
        ("1M", 30.0, curve.m1),
        ("3M", 91.0, curve.m3),
        ("6M", 182.0, curve.m6),
        ("1Y", 365.0, curve.y1),
        ("2Y", 730.0, curve.y2),
        ("5Y", 1_825.0, curve.y5),
        ("10Y", 3_650.0, curve.y10),
        ("30Y", 10_950.0, curve.y30),
    ]
    .into_iter()
    .filter_map(|(tenor, days, rate)| {
        rate.map(|rate| PublishedInterestRatePoint {
            tenor: tenor.to_string(),
            days,
            rate_percent: rate * 100.0,
        })
    })
    .collect()
}
