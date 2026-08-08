use chrono::NaiveDate;
use hexagonal_backend::hexagon::{
    PortResult,
    application::yield_curve_migration::YieldCurveMigrationApplication,
    domain::treasury::YieldCurve,
    driven_ports::{
        for_counting_yield_curves::ForCountingYieldCurves,
        for_loading_yield_curve_archive::ForLoadingYieldCurveArchive,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
    driving_ports::for_migrating_yield_curves::ForMigratingYieldCurves,
};

struct SourceMock;
struct TargetMock;

#[async_trait::async_trait]
impl ForLoadingYieldCurveArchive for SourceMock {
    async fn load_yield_curve_archive(&self) -> PortResult<Vec<YieldCurve>> {
        Ok(vec![YieldCurve {
            date: NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
            m1: None,
            m2: None,
            m3: None,
            m6: None,
            y1: None,
            y2: None,
            y3: None,
            y5: None,
            y7: None,
            y10: None,
            y20: None,
            y30: None,
        }])
    }
}

#[async_trait::async_trait]
impl ForStoringYieldCurves for TargetMock {
    async fn store_yield_curves(&self, curves: &[YieldCurve]) -> PortResult<u64> {
        Ok(curves.len() as u64)
    }
}

#[async_trait::async_trait]
impl ForCountingYieldCurves for TargetMock {
    async fn count_yield_curves(&self) -> PortResult<u64> {
        Ok(9_154)
    }
}

#[tokio::test]
async fn application_coordinates_yield_curve_migration_through_ports() {
    let report = YieldCurveMigrationApplication::new(SourceMock, TargetMock)
        .migrate_yield_curves()
        .await
        .expect("migration must succeed");
    assert_eq!(report.source_rows, 1);
    assert_eq!(report.target_rows, 9_154);
}
