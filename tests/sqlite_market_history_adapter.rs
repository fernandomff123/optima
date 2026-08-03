use polars_options::{
    driven_adapters::sqlite::index_history_port::SqliteIndexHistoryAdapter,
    driven_adapters::sqlite::market_history::{
        SqliteMarketHistoryAdapter, initialize, insert_incremental,
    },
    driven_adapters::yahoo::YahooLivePricesAdapter,
    hexagon::{
        application::market_data::MarketDataApplication, domain::market_history::MarketHistory,
        driving_ports::for_viewing_market_data::ForViewingMarketData,
    },
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn application_loads_market_history_through_the_sqlite_port() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    initialize(&pool).await.unwrap();
    insert_incremental(
        &pool,
        &MarketHistory {
            ticker: "AAPL".into(),
            currency: Some("USD".into()),
            exchange_timezone: Some("America/New_York".into()),
            daily_quotes: Vec::new(),
            dividends: Vec::new(),
            splits: Vec::new(),
        },
    )
    .await
    .unwrap();
    let app = MarketDataApplication::new(
        SqliteMarketHistoryAdapter::new(pool.clone()),
        SqliteIndexHistoryAdapter::new(pool),
        YahooLivePricesAdapter,
    );

    let history = app.market_history("aapl").await.unwrap();

    assert_eq!(history.ticker, "AAPL");
}
