use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use hexagonal_backend::hexagon::{
    PortResult,
    application::option_chain_migration::OptionChainMigrationApplication,
    domain::options::Snapshot,
    driven_ports::{
        for_counting_option_chains::{ForCountingOptionChains, OptionChainCounts},
        for_loading_option_chain_archive::{ArchivedOptionChain, ForLoadingOptionChainArchive},
        for_storing_option_chains::ForStoringOptionChains,
    },
    driving_ports::for_migrating_option_chains::ForMigratingOptionChains,
};

struct ArchiveMock(Vec<ArchivedOptionChain>);

#[async_trait::async_trait]
impl ForLoadingOptionChainArchive for ArchiveMock {
    async fn load_option_chain_archive(&self) -> PortResult<Vec<ArchivedOptionChain>> {
        Ok(self.0.clone())
    }
}

#[derive(Clone)]
struct TargetMock {
    stored: Arc<Mutex<Vec<String>>>,
    counts: OptionChainCounts,
}

#[async_trait::async_trait]
impl ForStoringOptionChains for TargetMock {
    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        _market_close: chrono::DateTime<Utc>,
    ) -> PortResult<u64> {
        self.stored
            .lock()
            .map_err(|error| hexagonal_backend::hexagon::PortError::Unavailable(error.to_string()))?
            .push(snapshot.ticker.clone());
        Ok(1)
    }
}

#[async_trait::async_trait]
impl ForCountingOptionChains for TargetMock {
    async fn count_option_chains(&self) -> PortResult<OptionChainCounts> {
        Ok(self.counts)
    }
}

fn archived(ticker: &str, has_market_close: bool) -> ArchivedOptionChain {
    let observed_at = Utc
        .with_ymd_and_hms(2026, 8, 7, 20, 0, 0)
        .single()
        .expect("test date must be valid");
    ArchivedOptionChain {
        snapshot: Snapshot {
            ticker: ticker.to_string(),
            timestamp_utc: observed_at,
            contratos: Vec::new(),
            chains: Vec::new(),
        },
        market_close: has_market_close.then_some(observed_at),
    }
}

#[tokio::test]
async fn application_coordinates_the_temporary_migration_through_ports() {
    let stored = Arc::new(Mutex::new(Vec::new()));
    let application = OptionChainMigrationApplication::new(
        ArchiveMock(vec![archived("SPX", true), archived("NDX", false)]),
        TargetMock {
            stored: Arc::clone(&stored),
            counts: OptionChainCounts {
                snapshots: 91,
                contracts: 779_232,
            },
        },
    );

    let report = application
        .migrate_option_chains()
        .await
        .expect("migration must succeed");

    assert_eq!(report.source_snapshots, 2);
    assert_eq!(report.inserted_snapshots, 1);
    assert_eq!(report.skipped_without_market_close, 1);
    assert_eq!(report.target_snapshots, 91);
    assert_eq!(report.target_contracts, 779_232);
    assert_eq!(
        *stored.lock().expect("stored tickers must be readable"),
        vec!["SPX"]
    );
}
