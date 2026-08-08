//! Coordinates the temporary offline migration of index histories.

use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_index_history::ForCountingIndexHistory,
        for_loading_index_history_archive::ForLoadingIndexHistoryArchive,
        for_storing_index_history::ForStoringIndexHistory,
    },
    driving_ports::for_migrating_index_history::{
        ForMigratingIndexHistory, IndexHistoryMigrationReport,
    },
};

pub struct IndexHistoryMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}

impl<Source, Target> IndexHistoryMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}

#[async_trait::async_trait]
impl<Source, Target> ForMigratingIndexHistory for IndexHistoryMigrationApplication<Source, Target>
where
    Source: ForLoadingIndexHistoryArchive,
    Target: ForStoringIndexHistory + ForCountingIndexHistory,
{
    async fn migrate_index_history(&self) -> PortResult<IndexHistoryMigrationReport> {
        let histories = self.source.load_index_history_archive().await?;
        let source_rows = histories
            .iter()
            .map(|history| history.daily_prices.len() as u64)
            .sum();
        for history in &histories {
            self.target.store_index_history(history).await?;
        }
        Ok(IndexHistoryMigrationReport {
            indices: histories.len() as u64,
            source_rows,
            target_rows: self.target.count_index_history().await?,
        })
    }
}
