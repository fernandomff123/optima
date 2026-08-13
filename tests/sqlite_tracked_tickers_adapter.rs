use hexagonal_backend::{
    driven_adapters::sqlite::tracked_tickers::SqliteTrackedTickersAdapter,
    hexagon::{
        application::tracked_tickers::TrackedTickersApplication,
        driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers,
    },
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn application_manages_tracked_tickers_through_sqlite_ports() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let adapter = SqliteTrackedTickersAdapter::new(pool);
    let application = TrackedTickersApplication::new(adapter.clone(), adapter);
    application
        .configure_ticker(
            "QQQ",
            hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
                active: true,
                historical_prices: true,
                option_snapshots: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(application.list_tickers(false).await.unwrap().len(), 1);
}
