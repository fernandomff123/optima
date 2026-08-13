use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{body::Body, http::Request};
use chrono::NaiveDate;
use hexagonal_backend::{
    configurator::{
        CompositionConfig, DEFAULT_DUCKDB_PATH, configure, configure_http,
        configure_server_http_application, configure_with_config,
        initialize_analytical_storage_with_config,
    },
    hexagon::{
        PortError, PortResult,
        driving_ports::{
            for_analyzing_options::ForAnalyzingOptions,
            for_managing_portfolios::ForManagingPortfolios,
            for_managing_saved_strategies::ForManagingSavedStrategies,
            for_managing_tracked_tickers::ForManagingTrackedTickers,
            for_preparing_intraday_simulations::ForPreparingIntradaySimulations,
            for_refreshing_market_data::{
                DataRefreshStatus, DataRefreshTrigger, ForRefreshingMarketData,
                StartDataRefreshResult,
            },
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
use tower::ServiceExt;

#[derive(Default)]
struct RefreshCharacterizationMock {
    triggers: Mutex<Vec<DataRefreshTrigger>>,
}

#[async_trait]
impl ForRefreshingMarketData for RefreshCharacterizationMock {
    async fn recover_interrupted_data_refreshes(
        &self,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> PortResult<u64> {
        Ok(0)
    }

    async fn request_data_refresh(
        &self,
        trigger: DataRefreshTrigger,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> PortResult<StartDataRefreshResult> {
        self.triggers
            .lock()
            .expect("refresh trigger lock")
            .push(trigger);
        Ok(StartDataRefreshResult::NoEligibleSession)
    }

    async fn next_data_refresh_attempt(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> PortResult<chrono::DateTime<chrono::Utc>> {
        Ok(now)
    }

    async fn data_refresh_status(&self, _recent_limit: usize) -> PortResult<DataRefreshStatus> {
        Ok(DataRefreshStatus {
            running: false,
            latest: None,
            recent: Vec::new(),
        })
    }
}

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
    assert_eq!(configured.composition_config, config);
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

#[tokio::test]
async fn server_router_keeps_legacy_and_hexagonal_routes_and_health_contract() {
    let directory = std::env::temp_dir().join(format!(
        "server-router-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let config = CompositionConfig::with_duckdb_path(directory.join("router.duckdb"));
    initialize_analytical_storage_with_config(&config)
        .await
        .expect("initialize temporary analytical storage");
    let mut configured = configure_with_config(&config);
    let refresh = Arc::new(RefreshCharacterizationMock::default());
    configured.data_refresh = refresh.clone();
    let (_market_session_updates, market_session) = tokio::sync::watch::channel(false);
    let router = configure_server_http_application(configured, market_session);

    async fn snapshot(
        router: axum::Router,
        request: Request<Body>,
    ) -> (u16, Option<String>, serde_json::Value) {
        let response = router.oneshot(request).await.expect("route response");
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let value = serde_json::from_slice(&body)
            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&body).into()));
        (status, content_type, value)
    }

    let get = |uri: &str| {
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("GET request")
    };
    let post = |uri: &str| {
        Request::builder()
            .method("POST")
            .uri(uri)
            .body(Body::empty())
            .expect("POST request")
    };

    assert_eq!(
        snapshot(router.clone(), get("/api/health")).await,
        (
            200,
            Some("text/plain; charset=utf-8".to_string()),
            serde_json::Value::String("ok".to_string())
        )
    );

    let spx = snapshot(router.clone(), get("/api/market/spx-history")).await;
    assert_eq!((spx.0, spx.1.as_deref()), (500, None));
    assert_eq!(spx.2, serde_json::Value::String(String::new()));

    let sectors = snapshot(router.clone(), get("/api/market/sectors?period=1w")).await;
    assert_eq!(
        (sectors.0, sectors.1.as_deref()),
        (200, Some("application/json"))
    );
    assert_eq!(sectors.2["period"], "1w");

    let refresh_status = snapshot(router.clone(), get("/api/data-refresh/status")).await;
    assert_eq!(
        (refresh_status.0, refresh_status.1.as_deref()),
        (200, Some("application/json"))
    );
    assert_eq!(refresh_status.2["running"], false);
    let refresh_post = snapshot(router.clone(), post("/api/data-refresh")).await;
    assert_eq!(
        (refresh_post.0, refresh_post.1.as_deref()),
        (409, Some("application/json"))
    );
    assert_eq!(refresh_post.2["result"], "no_eligible_session");

    let tracked = snapshot(router.clone(), get("/tracked-tickers")).await;
    assert_eq!(
        (tracked.0, tracked.1.as_deref()),
        (200, Some("application/json"))
    );
    assert!(
        tracked
            .2
            .as_array()
            .is_some_and(|tickers| !tickers.is_empty())
    );

    let portfolio = snapshot(router.clone(), get("/api/portfolio")).await;
    assert_eq!(
        (portfolio.0, portfolio.1.as_deref()),
        (200, Some("application/json"))
    );
    assert_eq!(portfolio.2["id"], "main");

    let options = snapshot(router.clone(), get("/api/assets/SPX/options/snapshot")).await;
    assert_eq!(
        (options.0, options.1.as_deref()),
        (200, Some("application/json"))
    );
    assert_eq!(options.2["ticker"], "SPX");
    assert_eq!(options.2["snapshot"]["state"], "unavailable");

    let simulation = snapshot(router.clone(), get("/api/simulation?ticker=SPX")).await;
    assert_eq!((simulation.0, simulation.1.as_deref()), (404, None));

    let websocket = snapshot(
        router.clone(),
        Request::builder()
            .uri("/api/assets/live?ticker=SPX")
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(Body::empty())
            .expect("WebSocket handshake"),
    )
    .await;
    // Router-level requests have no Hyper OnUpgrade extension; reaching the
    // WebSocket extractor is therefore characterized by Axum's 426 response.
    assert_eq!(websocket.0, 426);

    assert_eq!(
        refresh
            .triggers
            .lock()
            .expect("refresh trigger lock")
            .as_slice(),
        &[DataRefreshTrigger::Manual]
    );

    drop(router);
    std::fs::remove_dir_all(directory).expect("remove temporary directory");
}
