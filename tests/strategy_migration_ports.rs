use chrono::Utc;
use hexagonal_backend::hexagon::{
    PortResult,
    application::strategy_migration::StrategyMigrationApplication,
    domain::saved_strategy::SavedStrategy,
    driven_ports::{
        for_counting_strategies::{ForCountingStrategies, StrategyCounts},
        for_importing_strategy_archive::ForImportingStrategyArchive,
        for_loading_strategies::ForLoadingStrategies,
    },
    driving_ports::for_migrating_strategies::ForMigratingStrategies,
};

struct SourceMock;
struct TargetMock;

#[async_trait::async_trait]
impl ForLoadingStrategies for SourceMock {
    async fn load_strategies(&self) -> PortResult<Vec<SavedStrategy>> {
        Ok(vec![SavedStrategy {
            id: 1,
            name: "Empty".into(),
            ticker: "SPY".into(),
            legs: Vec::new(),
            updated_at: Utc::now(),
        }])
    }
}

#[async_trait::async_trait]
impl ForImportingStrategyArchive for TargetMock {
    async fn import_strategy(&self, _strategy: &SavedStrategy) -> PortResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ForCountingStrategies for TargetMock {
    async fn count_strategies(&self) -> PortResult<StrategyCounts> {
        Ok(StrategyCounts {
            strategies: 1,
            legs: 0,
        })
    }
}

#[tokio::test]
async fn application_coordinates_strategy_migration_through_ports() {
    let report = StrategyMigrationApplication::new(SourceMock, TargetMock)
        .migrate_strategies()
        .await
        .expect("migration must succeed");
    assert_eq!(report.source, report.target);
}
