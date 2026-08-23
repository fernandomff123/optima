use async_trait::async_trait;
use chrono::NaiveDate;
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::interest_rates::InterestRatesApplication,
    domain::{interest_rates::BoundedCubicSpline, treasury::YieldCurve},
    driven_ports::for_loading_yield_curves::ForLoadingYieldCurves,
    driving_ports::for_viewing_interest_rates::ForViewingInterestRates,
};

struct YieldCurvesMock(PortResult<Option<YieldCurve>>);

#[async_trait]
impl ForLoadingYieldCurves for YieldCurvesMock {
    async fn load_yield_curve(&self, _on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        self.0.clone()
    }
}

fn curve(date: NaiveDate) -> YieldCurve {
    YieldCurve {
        date,
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
    }
}

#[tokio::test]
async fn views_a_yield_curve_through_a_mocked_driven_port() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid test date");
    let application = InterestRatesApplication::new(YieldCurvesMock(Ok(Some(curve(date)))));

    let curve = application
        .yield_curve(date)
        .await
        .expect("port call must succeed")
        .expect("mock supplies a curve");

    assert_eq!(curve.date, date);
    assert_eq!(curve.m1, Some(0.04));
}

#[tokio::test]
async fn application_builds_the_same_ordered_360_month_bey_projection() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid test date");
    let source = curve(date);
    let spline = BoundedCubicSpline::from_treasury_curve(&source).expect("valid test curve");
    let application = InterestRatesApplication::new(YieldCurvesMock(Ok(Some(source))));

    let projection = application
        .interest_rate_curve(date)
        .await
        .expect("projection call must succeed")
        .expect("mock supplies a curve");

    assert_eq!(projection.date, date);
    assert_eq!(projection.published_points.len(), 2);
    assert_eq!(projection.published_points[0].tenor, "1M");
    assert_eq!(projection.published_points[0].rate_percent, 4.0);
    assert_eq!(projection.published_points[1].tenor, "3M");
    assert_eq!(
        projection.published_points[1].rate_percent,
        4.1000000000000005
    );
    assert_eq!(projection.interpolated_points.len(), 360);
    for (index, point) in projection.interpolated_points.iter().enumerate() {
        let days = (index + 1) as f64 * 30.0;
        assert_eq!(point.days, days);
        assert_eq!(
            point.rate_percent,
            spline
                .bond_equivalent_yield(days)
                .expect("grid maturity must be valid")
                * 100.0
        );
    }
}

#[tokio::test]
async fn absent_and_invalid_curves_keep_their_distinct_application_results() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid test date");
    let absent = InterestRatesApplication::new(YieldCurvesMock(Ok(None)));
    assert_eq!(absent.interest_rate_curve(date).await.unwrap(), None);

    let mut incomplete = curve(date);
    incomplete.m3 = None;
    let application = InterestRatesApplication::new(YieldCurvesMock(Ok(Some(incomplete))));
    assert!(matches!(
        application.interest_rate_curve(date).await.unwrap_err(),
        PortError::Unavailable(_)
    ));

    let mut invalid = curve(date);
    invalid.m1 = Some(f64::NAN);
    let application = InterestRatesApplication::new(YieldCurvesMock(Ok(Some(invalid))));
    assert!(matches!(
        application.interest_rate_curve(date).await.unwrap_err(),
        PortError::Unavailable(_)
    ));
}

#[tokio::test]
async fn loading_errors_are_propagated_without_reclassification() {
    let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid test date");
    for expected in [
        PortError::InvalidRequest("invalid".into()),
        PortError::NotFound("missing".into()),
        PortError::Conflict("conflict".into()),
        PortError::Unavailable("storage".into()),
    ] {
        let application = InterestRatesApplication::new(YieldCurvesMock(Err(expected.clone())));
        assert_eq!(
            application.interest_rate_curve(date).await.unwrap_err(),
            expected
        );
    }
}
