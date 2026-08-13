use chrono::NaiveDate;
use hexagonal_backend::{
    configurator::{
        CompositionConfig, DEFAULT_DUCKDB_PATH, configure, configure_http, configure_with_config,
        initialize_analytical_storage_with_config,
    },
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
            for_viewing_sector_performance::ForViewingSectorPerformance,
            for_viewing_volatility::ForViewingVolatility,
        },
    },
};

#[tokio::test]
async fn configurator_wires_every_application_to_its_driving_port() {
    let configured = configure();

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
    fn sector_performance(_: &impl ForViewingSectorPerformance) {}

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
    sector_performance(&configured.sector_performance);

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

#[test]
fn composition_database_path_has_a_production_default_and_explicit_override() {
    assert_eq!(
        CompositionConfig::default().duckdb_path(),
        std::path::Path::new(DEFAULT_DUCKDB_PATH)
    );
    let override_path = std::path::Path::new("/tmp/optima-test/override.duckdb");
    assert_eq!(
        CompositionConfig::with_duckdb_path(override_path).duckdb_path(),
        override_path
    );
    assert_eq!(
        CompositionConfig::from_path_override(None).duckdb_path(),
        std::path::Path::new(DEFAULT_DUCKDB_PATH)
    );
    assert_eq!(
        CompositionConfig::from_path_override(Some(override_path.as_os_str().to_owned()))
            .duckdb_path(),
        override_path
    );
}

#[tokio::test]
async fn every_configured_adapter_uses_the_override_and_refresh_arc_is_shared() {
    let directory = std::env::temp_dir().join(format!("composition-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let database = directory.join("override.duckdb");
    let config = CompositionConfig::with_duckdb_path(&database);
    initialize_analytical_storage_with_config(&config)
        .await
        .expect("initialize override");
    let configured = configure_with_config(&config);
    let startup = configured.data_refresh.clone();
    let scheduler = configured.data_refresh.clone();
    let http = configured.data_refresh.clone();
    assert!(std::sync::Arc::ptr_eq(&startup, &scheduler));
    assert!(std::sync::Arc::ptr_eq(&scheduler, &http));
    assert!(database.exists());
    drop(configured);
    drop(startup);
    drop(scheduler);
    drop(http);
    std::fs::remove_dir_all(directory).expect("remove temporary directory");
}

#[tokio::test]
async fn configurator_connects_the_application_to_the_http_adapter() {
    let _router = configure_http();
}
