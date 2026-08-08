//! Coordinates offline migration of tracked ticker configuration.
use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_tracked_tickers::ForCountingTrackedTickers,
        for_loading_tracked_ticker_archive::ForLoadingTrackedTickerArchive,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
    driving_ports::for_migrating_tracked_tickers::{
        ForMigratingTrackedTickers, TrackedTickerMigrationReport,
    },
};
pub struct TrackedTickerMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}
impl<Source, Target> TrackedTickerMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}
#[async_trait::async_trait]
impl<Source, Target> ForMigratingTrackedTickers
    for TrackedTickerMigrationApplication<Source, Target>
where
    Source: ForLoadingTrackedTickerArchive,
    Target: ForStoringTrackedTickers + ForCountingTrackedTickers,
{
    async fn migrate_tracked_tickers(&self) -> PortResult<TrackedTickerMigrationReport> {
        let tickers = self.source.load_tracked_ticker_archive().await?;
        for ticker in &tickers {
            self.target.store_tracked_ticker(ticker).await?;
        }
        Ok(TrackedTickerMigrationReport {
            source_rows: tickers.len() as u64,
            target_rows: self.target.count_tracked_tickers().await?,
        })
    }
}
