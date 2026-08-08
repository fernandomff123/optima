use hexagonal_backend::hexagon::{
    PortResult,
    application::tracked_ticker_migration::TrackedTickerMigrationApplication,
    domain::tracked_ticker::TrackedTicker,
    driven_ports::{
        for_counting_tracked_tickers::ForCountingTrackedTickers,
        for_loading_tracked_ticker_archive::ForLoadingTrackedTickerArchive,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
    driving_ports::for_migrating_tracked_tickers::ForMigratingTrackedTickers,
};
struct SourceMock;
struct TargetMock;
#[async_trait::async_trait]
impl ForLoadingTrackedTickerArchive for SourceMock {
    async fn load_tracked_ticker_archive(&self) -> PortResult<Vec<TrackedTicker>> {
        Ok(Vec::new())
    }
}
#[async_trait::async_trait]
impl ForStoringTrackedTickers for TargetMock {
    async fn store_tracked_ticker(&self, _ticker: &TrackedTicker) -> PortResult<()> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl ForCountingTrackedTickers for TargetMock {
    async fn count_tracked_tickers(&self) -> PortResult<u64> {
        Ok(7)
    }
}
#[tokio::test]
async fn application_coordinates_tracked_ticker_migration_through_ports() {
    let report = TrackedTickerMigrationApplication::new(SourceMock, TargetMock)
        .migrate_tracked_tickers()
        .await
        .expect("migration must succeed");
    assert_eq!(report.target_rows, 7);
}
