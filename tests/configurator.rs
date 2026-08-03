use chrono::NaiveDate;
use polars_options::{
    configurator::{configure, configure_http},
    hexagon::{
        PortError,
        driving_ports::{
            for_analyzing_options::ForAnalyzingOptions,
            for_managing_portfolios::ForManagingPortfolios,
            for_managing_saved_strategies::ForManagingSavedStrategies,
            for_managing_tracked_tickers::ForManagingTrackedTickers,
            for_preparing_intraday_simulations::ForPreparingIntradaySimulations,
            for_scheduling_market_operations::ForSchedulingMarketOperations,
            for_simulating_strategies::{ForSimulatingStrategies, ScenarioGridRequest},
            for_streaming_market_prices::ForStreamingMarketPrices,
            for_synchronizing_market_data::ForSynchronizingMarketData,
            for_viewing_interest_rates::ForViewingInterestRates,
            for_viewing_intraday_options::ForViewingIntradayOptions,
            for_viewing_market_data::ForViewingMarketData,
            for_viewing_portfolio_positions::ForViewingPortfolioPositions,
            for_viewing_volatility::ForViewingVolatility,
        },
    },
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn configurator_wires_every_application_to_its_driving_port() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite must connect");
    let configured = configure(pool);

    fn market_data(_: &impl ForViewingMarketData) {}
    fn market_stream(_: &impl ForStreamingMarketPrices) {}
    fn market_scheduling(_: &impl ForSchedulingMarketOperations) {}
    fn interest_rates(_: &impl ForViewingInterestRates) {}
    fn market_volatility(_: &impl ForViewingVolatility) {}
    fn portfolio_valuation(_: &impl ForViewingPortfolioPositions) {}
    fn intraday_simulation(_: &impl ForPreparingIntradaySimulations) {}
    fn intraday_options(_: &impl ForViewingIntradayOptions) {}
    fn options(_: &impl ForAnalyzingOptions) {}
    fn portfolios(_: &impl ForManagingPortfolios) {}
    fn saved_strategies(_: &impl ForManagingSavedStrategies) {}
    fn tracked_tickers(_: &impl ForManagingTrackedTickers) {}
    fn simulation(_: &impl ForSimulatingStrategies) {}
    fn synchronization(_: &impl ForSynchronizingMarketData) {}

    market_data(&configured.market_data);
    market_stream(&configured.market_stream);
    market_scheduling(&configured.market_scheduling);
    interest_rates(&configured.interest_rates);
    market_volatility(&configured.market_volatility);
    portfolio_valuation(&configured.portfolio_valuation);
    intraday_simulation(&configured.intraday_simulation);
    intraday_options(&configured.intraday_simulation);
    options(&configured.options);
    portfolios(&configured.portfolios);
    saved_strategies(&configured.saved_strategies);
    tracked_tickers(&configured.tracked_tickers);
    simulation(&configured.simulation);
    synchronization(&configured.synchronization);

    let error = configured
        .simulation
        .build_scenario_grid(ScenarioGridRequest {
            spot: 0.0,
            range_fraction: 0.1,
            spot_count: 3,
            valuation_dates: vec![NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid test date")],
            volatility_shifts: vec![0.0],
            required_spots: Vec::new(),
        })
        .await
        .expect_err("configured application must execute use-case validation");
    assert!(matches!(error, PortError::InvalidRequest(_)));
}

#[tokio::test]
async fn configurator_connects_the_application_to_the_http_adapter() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite must connect");

    let _router = configure_http(pool);
}
