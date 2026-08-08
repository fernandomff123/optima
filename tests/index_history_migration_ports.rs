use std::sync::{Arc, Mutex};

use hexagonal_backend::hexagon::{
    PortResult,
    application::index_history_migration::IndexHistoryMigrationApplication,
    domain::index_history::IndexHistory,
    driven_ports::{
        for_counting_index_history::ForCountingIndexHistory,
        for_loading_index_history_archive::ForLoadingIndexHistoryArchive,
        for_storing_index_history::ForStoringIndexHistory,
    },
    driving_ports::for_migrating_index_history::ForMigratingIndexHistory,
};

struct SourceMock(Vec<IndexHistory>);

#[async_trait::async_trait]
impl ForLoadingIndexHistoryArchive for SourceMock {
    async fn load_index_history_archive(&self) -> PortResult<Vec<IndexHistory>> {
        Ok(self.0.clone())
    }
}

struct TargetMock(Arc<Mutex<Vec<String>>>);

#[async_trait::async_trait]
impl ForStoringIndexHistory for TargetMock {
    async fn store_index_history(&self, history: &IndexHistory) -> PortResult<u64> {
        self.0
            .lock()
            .map_err(|error| hexagonal_backend::hexagon::PortError::Unavailable(error.to_string()))?
            .push(history.ticker.clone());
        Ok(0)
    }
}

#[async_trait::async_trait]
impl ForCountingIndexHistory for TargetMock {
    async fn count_index_history(&self) -> PortResult<u64> {
        Ok(194_317)
    }
}

#[tokio::test]
async fn application_coordinates_index_migration_through_ports() {
    let stored = Arc::new(Mutex::new(Vec::new()));
    let application = IndexHistoryMigrationApplication::new(
        SourceMock(vec![IndexHistory {
            ticker: "VIX".to_string(),
            daily_prices: Vec::new(),
        }]),
        TargetMock(Arc::clone(&stored)),
    );

    let report = application
        .migrate_index_history()
        .await
        .expect("migration must succeed");

    assert_eq!(report.indices, 1);
    assert_eq!(report.target_rows, 194_317);
    assert_eq!(
        *stored.lock().expect("stored indices must be readable"),
        vec!["VIX"]
    );
}
