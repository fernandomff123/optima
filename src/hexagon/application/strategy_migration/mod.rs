//! Coordinates offline migration of saved strategy definitions.

use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_strategies::{ForCountingStrategies, StrategyCounts},
        for_importing_strategy_archive::ForImportingStrategyArchive,
        for_loading_strategies::ForLoadingStrategies,
    },
    driving_ports::for_migrating_strategies::{ForMigratingStrategies, StrategyMigrationReport},
};

pub struct StrategyMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}

impl<Source, Target> StrategyMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}

#[async_trait::async_trait]
impl<Source, Target> ForMigratingStrategies for StrategyMigrationApplication<Source, Target>
where
    Source: ForLoadingStrategies,
    Target: ForImportingStrategyArchive + ForCountingStrategies,
{
    async fn migrate_strategies(&self) -> PortResult<StrategyMigrationReport> {
        let strategies = self.source.load_strategies().await?;
        let source = StrategyCounts {
            strategies: strategies.len() as u64,
            legs: strategies
                .iter()
                .map(|strategy| strategy.legs.len() as u64)
                .sum(),
        };
        for strategy in &strategies {
            self.target.import_strategy(strategy).await?;
        }
        Ok(StrategyMigrationReport {
            source,
            target: self.target.count_strategies().await?,
        })
    }
}
