use hexagonal_backend::{
    driven_adapters::{
        duckdb::tracked_tickers::DuckDbTrackedTickersAdapter,
        sqlite::{tracked_tickers, tracked_tickers::SqliteTrackedTickersAdapter},
    },
    hexagon::{
        PortError,
        application::tracked_tickers::TrackedTickersApplication,
        domain::tracked_ticker::{TrackedTicker, TrackedTickerConfiguration, TrackedTickerSource},
        driven_ports::{
            for_loading_tracked_tickers::ForLoadingTrackedTickers,
            for_storing_tracked_tickers::ForStoringTrackedTickers,
        },
        driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers,
    },
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::atomic::{AtomicU64, Ordering};
static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn assert_contract(adapter: &(impl ForLoadingTrackedTickers + ForStoringTrackedTickers)) {
    let ticker = TrackedTicker {
        ticker: "QQQ".to_string(),
        source: hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::User,
        active: true,
        historical_prices: true,
        option_snapshots: false,
    };
    adapter
        .store_tracked_ticker(&ticker)
        .await
        .expect("ticker must store");
    assert!(
        adapter
            .load_active_tickers()
            .await
            .expect("tickers must load")
            .contains(&ticker)
    );
    let mut inactive = ticker.clone();
    inactive.active = false;
    adapter
        .store_tracked_ticker(&inactive)
        .await
        .expect("ticker must update");
    assert!(
        !adapter
            .load_active_tickers()
            .await
            .expect("tickers must load")
            .iter()
            .any(|item| item.ticker == "QQQ")
    );
    assert_eq!(
        adapter
            .load_tracked_tickers()
            .await
            .expect("complete catalog must load"),
        vec![inactive]
    );
}

#[tokio::test]
async fn sqlite_satisfies_tracked_ticker_contract() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("SQLite must open");
    tracked_tickers::initialize(&pool)
        .await
        .expect("schema must initialize");
    assert_contract(&SqliteTrackedTickersAdapter::new(pool)).await;
}

#[tokio::test]
async fn duckdb_satisfies_tracked_ticker_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-tracked-tickers-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}

#[tokio::test]
async fn duckdb_supports_the_complete_tracked_ticker_lifecycle() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-sector-tickers-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert!(adapter.load_tracked_tickers().await.unwrap().is_empty());
    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone());
    application.bootstrap_system_tickers().await.unwrap();
    application.bootstrap_system_tickers().await.unwrap();

    let active = adapter
        .load_active_tickers()
        .await
        .expect("tickers must load");
    for ticker in hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers() {
        let tracked = active
            .iter()
            .find(|item| item.ticker == ticker.ticker)
            .expect("default must exist");
        assert_eq!(tracked, &ticker);
    }
    assert_eq!(active.len(), 14);

    let system = active.iter().find(|ticker| ticker.ticker == "SPX").unwrap();
    application
        .configure_ticker("spx", system.configuration())
        .await
        .expect("identical system configuration must be idempotent");
    let forbidden = TrackedTickerConfiguration {
        active: false,
        ..system.configuration()
    };
    assert_eq!(
        application.configure_ticker("SPX", forbidden).await,
        Err(PortError::Conflict(
            "tracked ticker SPX is protected by the system".into()
        ))
    );
    assert_eq!(
        application
            .configure_ticker(
                "bad ticker",
                TrackedTickerConfiguration {
                    active: true,
                    historical_prices: true,
                    option_snapshots: false,
                },
            )
            .await,
        Err(PortError::InvalidRequest("invalid tracked ticker".into()))
    );

    let enabled = TrackedTickerConfiguration {
        active: true,
        historical_prices: true,
        option_snapshots: false,
    };
    application
        .configure_ticker(" qqq ", enabled)
        .await
        .unwrap();
    assert!(
        adapter
            .load_active_tickers()
            .await
            .unwrap()
            .iter()
            .any(|ticker| ticker.ticker == "QQQ")
    );
    let updated = TrackedTickerConfiguration {
        option_snapshots: true,
        ..enabled
    };
    application.configure_ticker("QQQ", updated).await.unwrap();
    let disabled = TrackedTickerConfiguration {
        active: false,
        ..updated
    };
    application.configure_ticker("QQQ", disabled).await.unwrap();
    assert!(
        !adapter
            .load_active_tickers()
            .await
            .unwrap()
            .iter()
            .any(|ticker| ticker.ticker == "QQQ")
    );
    let stored = adapter.load_tracked_tickers().await.unwrap();
    assert!(
        stored
            .iter()
            .any(|ticker| ticker.ticker == "QQQ" && !ticker.active)
    );
    application.configure_ticker("QQQ", updated).await.unwrap();
    let refreshed = adapter.load_active_tickers().await.unwrap();
    assert!(
        refreshed
            .iter()
            .any(|ticker| ticker.ticker == "QQQ" && ticker.option_snapshots)
    );
    assert_eq!(
        refreshed
            .iter()
            .filter(|ticker| ticker.ticker == "QQQ")
            .count(),
        1
    );
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}

#[tokio::test]
async fn duckdb_migrates_the_legacy_schema_idempotently() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-legacy-tracked-tickers-{}-{sequence}.duckdb",
        std::process::id()
    ));
    {
        let connection = duckdb::Connection::open(&path).expect("DuckDB must open");
        connection
            .execute_batch(
                "CREATE TABLE tracked_tickers (
                    ticker VARCHAR PRIMARY KEY,
                    active BOOLEAN NOT NULL,
                    historical_prices BOOLEAN NOT NULL,
                    option_snapshots BOOLEAN NOT NULL
                );
                INSERT INTO tracked_tickers VALUES ('QQQ', false, true, false);
                INSERT INTO tracked_tickers VALUES ('SPX', false, false, false);",
            )
            .expect("legacy schema must be created");
    }
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.expect("migration must succeed");
    adapter
        .initialize()
        .await
        .expect("migration must be idempotent");

    let catalog = adapter
        .load_tracked_tickers()
        .await
        .expect("catalog must load");
    assert_eq!(catalog.len(), 2);
    let qqq = catalog
        .iter()
        .find(|ticker| ticker.ticker == "QQQ")
        .unwrap();
    assert_eq!(qqq.source, TrackedTickerSource::User);
    assert!(!qqq.active);

    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone());
    application.bootstrap_system_tickers().await.unwrap();
    application.bootstrap_system_tickers().await.unwrap();
    let promoted = adapter.load_tracked_tickers().await.unwrap();
    assert_eq!(promoted.len(), 15);
    assert_eq!(
        promoted
            .iter()
            .find(|ticker| ticker.ticker == "SPX")
            .unwrap(),
        &hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers()
            .into_iter()
            .find(|ticker| ticker.ticker == "SPX")
            .unwrap()
    );
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
