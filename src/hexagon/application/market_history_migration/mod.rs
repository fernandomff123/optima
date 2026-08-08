//! Coordinates the temporary offline migration of market histories.

use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_market_history::{ForCountingMarketHistory, MarketHistoryCounts},
        for_loading_market_history_archive::ForLoadingMarketHistoryArchive,
        for_storing_market_history::ForStoringMarketHistory,
    },
    driving_ports::for_migrating_market_history::{
        ForMigratingMarketHistory, MarketHistoryMigrationReport,
    },
};

pub struct MarketHistoryMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}

impl<Source, Target> MarketHistoryMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}

#[async_trait::async_trait]
impl<Source, Target> ForMigratingMarketHistory for MarketHistoryMigrationApplication<Source, Target>
where
    Source: ForLoadingMarketHistoryArchive,
    Target: ForStoringMarketHistory + ForCountingMarketHistory,
{
    async fn migrate_market_history(&self) -> PortResult<MarketHistoryMigrationReport> {
        let histories = self.source.load_market_history_archive().await?;
        let source = MarketHistoryCounts {
            prices: histories
                .iter()
                .map(|history| history.daily_quotes.len() as u64)
                .sum(),
            dividends: histories
                .iter()
                .map(|history| history.dividends.len() as u64)
                .sum(),
            splits: histories
                .iter()
                .map(|history| history.splits.len() as u64)
                .sum(),
        };
        for history in &histories {
            self.target.store_market_history(history).await?;
        }
        let target = self.target.count_market_history().await?;
        Ok(MarketHistoryMigrationReport {
            histories: histories.len() as u64,
            source,
            target,
        })
    }
}
