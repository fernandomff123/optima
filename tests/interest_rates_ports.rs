use async_trait::async_trait;
use chrono::NaiveDate;
use polars_options::hexagon::{
    PortResult, application::interest_rates::InterestRatesApplication,
    domain::treasury::YieldCurve, driven_ports::for_loading_yield_curves::ForLoadingYieldCurves,
    driving_ports::for_viewing_interest_rates::ForViewingInterestRates,
};

struct YieldCurvesMock;

#[async_trait]
impl ForLoadingYieldCurves for YieldCurvesMock {
    async fn load_yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        Ok(Some(YieldCurve {
            date: on_or_before,
            m1: Some(0.04),
            m2: None,
            m3: Some(0.041),
            m6: None,
            y1: None,
            y2: None,
            y3: None,
            y5: None,
            y7: None,
            y10: None,
            y20: None,
            y30: None,
        }))
    }
}

#[tokio::test]
async fn views_a_yield_curve_through_a_mocked_driven_port() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid test date");
    let application = InterestRatesApplication::new(YieldCurvesMock);

    let curve = application
        .yield_curve(date)
        .await
        .expect("port call must succeed")
        .expect("mock supplies a curve");

    assert_eq!(curve.date, date);
    assert_eq!(curve.m1, Some(0.04));
}
