use hexagonal_backend::{
    driven_adapters::{
        duckdb::tracked_tickers::DuckDbTrackedTickersAdapter,
        sqlite::{tracked_tickers, tracked_tickers::SqliteTrackedTickersAdapter},
    },
    hexagon::{
        domain::tracked_ticker::TrackedTicker,
        driven_ports::{
            for_loading_tracked_tickers::ForLoadingTrackedTickers,
            for_storing_tracked_tickers::ForStoringTrackedTickers,
        },
    },
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::atomic::{AtomicU64, Ordering};
static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn assert_contract(adapter: &(impl ForLoadingTrackedTickers + ForStoringTrackedTickers)) {
    let ticker = TrackedTicker {
        ticker: "QQQ".to_string(),
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
