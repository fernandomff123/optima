//! Opt-in live validation. It never opens the configured production database.

use std::sync::atomic::{AtomicU64, Ordering};

use hexagonal_backend::{
    driven_adapters::{
        duckdb::tracked_tickers::DuckDbTrackedTickersAdapter, yahoo::YahooUnderlyingResolverAdapter,
    },
    hexagon::{
        PortError,
        application::tracked_tickers::TrackedTickersApplication,
        domain::tracked_ticker::{TrackedTicker, TrackedTickerConfiguration},
        driven_ports::{
            for_loading_tracked_tickers::ForLoadingTrackedTickers,
            for_storing_tracked_tickers::ForStoringTrackedTickers,
        },
        driving_ports::{
            for_managing_tracked_tickers::ForManagingTrackedTickers,
            for_resolving_underlyings::ForResolvingUnderlyings,
        },
    },
};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
#[ignore = "opt-in live Yahoo validation using a temporary DuckDB"]
async fn validates_exact_resolution_and_put_without_running_a_global_refresh() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-live-resolution-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.unwrap();
    let application = TrackedTickersApplication::new(
        adapter.clone(),
        adapter.clone(),
        YahooUnderlyingResolverAdapter::default(),
    );

    for ticker in ["MSFT", "BRK.B", "SPX"] {
        assert_eq!(
            application.resolve_underlying(ticker).await.unwrap().ticker,
            ticker
        );
    }

    let configuration = TrackedTickerConfiguration {
        active: true,
        historical_prices: true,
        option_snapshots: false,
    };
    application
        .configure_ticker("MSFT", configuration)
        .await
        .unwrap();
    assert!(
        application
            .list_tickers(false)
            .await
            .unwrap()
            .iter()
            .any(|ticker| ticker.ticker == "MSFT")
    );

    let missing = "ZZZZZZZZZZZZZZZ";
    assert!(matches!(
        application.configure_ticker(missing, configuration).await,
        Err(PortError::NotFound(_))
    ));
    assert!(
        adapter
            .load_tracked_tickers()
            .await
            .unwrap()
            .iter()
            .all(|ticker| ticker.ticker != missing)
    );

    let mut rejected = TrackedTicker::user("REJECTED", configuration).unwrap();
    rejected.reject();
    adapter.store_tracked_ticker(&rejected).await.unwrap();
    adapter
        .store_tracked_ticker(&TrackedTicker::user("PENDING", configuration).unwrap())
        .await
        .unwrap();
    let eligible = adapter.load_refresh_eligible_tickers().await.unwrap();
    assert!(eligible.iter().any(|ticker| ticker.ticker == "MSFT"));
    assert!(!eligible.iter().any(|ticker| ticker.ticker == "PENDING"));
    assert!(!eligible.iter().any(|ticker| ticker.ticker == "REJECTED"));

    std::fs::remove_file(path).unwrap();
}
