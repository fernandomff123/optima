use std::sync::{Arc, Mutex};

use hexagonal_backend::hexagon::{
    PortResult,
    application::market_history_migration::MarketHistoryMigrationApplication,
    domain::market_history::MarketHistory,
    driven_ports::{
        for_counting_market_history::{ForCountingMarketHistory, MarketHistoryCounts},
        for_loading_market_history_archive::ForLoadingMarketHistoryArchive,
        for_storing_market_history::ForStoringMarketHistory,
    },
    driving_ports::for_migrating_market_history::ForMigratingMarketHistory,
};

struct ArchiveMock(Vec<MarketHistory>);

#[async_trait::async_trait]
impl ForLoadingMarketHistoryArchive for ArchiveMock {
    async fn load_market_history_archive(&self) -> PortResult<Vec<MarketHistory>> {
        Ok(self.0.clone())
    }
}

struct TargetMock {
    stored: Arc<Mutex<Vec<String>>>,
    counts: MarketHistoryCounts,
}

#[async_trait::async_trait]
impl ForStoringMarketHistory for TargetMock {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64> {
        self.stored
            .lock()
            .map_err(|error| hexagonal_backend::hexagon::PortError::Unavailable(error.to_string()))?
            .push(history.ticker.clone());
        Ok(0)
    }
}

#[async_trait::async_trait]
impl ForCountingMarketHistory for TargetMock {
    async fn count_market_history(&self) -> PortResult<MarketHistoryCounts> {
        Ok(self.counts)
    }
}

#[tokio::test]
async fn application_coordinates_market_history_migration_through_ports() {
    let stored = Arc::new(Mutex::new(Vec::new()));
    let history = MarketHistory {
        ticker: "SPX".to_string(),
        currency: None,
        exchange_timezone: None,
        daily_quotes: Vec::new(),
        dividends: Vec::new(),
        splits: Vec::new(),
    };
    let application = MarketHistoryMigrationApplication::new(
        ArchiveMock(vec![history]),
        TargetMock {
            stored: Arc::clone(&stored),
            counts: MarketHistoryCounts {
                prices: 75_863,
                dividends: 721,
                splits: 25,
            },
        },
    );

    let report = application
        .migrate_market_history()
        .await
        .expect("migration must succeed");

    assert_eq!(report.histories, 1);
    assert_eq!(report.target.prices, 75_863);
    assert_eq!(
        *stored.lock().expect("stored histories must be readable"),
        vec!["SPX"]
    );
}
