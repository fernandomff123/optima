use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, NaiveDate, Utc};
use hexagonal_backend::{
    driving_adapters::http,
    hexagon::{
        PortError, PortResult,
        domain::{
            index_history::IndexHistory,
            live_price::LivePrice,
            market_history::MarketHistory,
            options::Snapshot,
            portfolio::{
                CashMovement, Currency, CurrencyExchange, Portfolio, PortfolioEvent, Position,
                Trade,
            },
            saved_strategy::SavedStrategy,
            simulation::{
                Greeks, ScenarioGrid, SimulationRequest, SimulationResult, SimulationScenario,
            },
            tracked_ticker::{TrackedTicker, UnderlyingMetadata, UnderlyingResolutionState},
            volatility::TermStructure,
            volatility_surface::{VolatilitySkew, VolatilitySurface},
        },
        driving_ports::{
            for_analyzing_options::{ForAnalyzingOptions, GreeksRequest},
            for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
            for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
            for_managing_tracked_tickers::ForManagingTrackedTickers,
            for_resolving_underlyings::{ForResolvingUnderlyings, UnderlyingResolution},
            for_simulating_strategies::{
                ForSimulatingStrategies, ScenarioGridRequest, SimulateScenario,
            },
            for_synchronizing_market_data::{
                ForSynchronizingMarketData, SynchronizationReport, SynchronizeTrackedTickers,
                TrackedTickersSynchronizationReport,
            },
            for_viewing_market_data::ForViewingMarketData,
            for_viewing_sector_performance::ForViewingSectorPerformance,
        },
    },
};
use rust_decimal::Decimal;
use tower::ServiceExt;

#[derive(Default)]
struct MarketDataMock {
    requested: Mutex<Vec<String>>,
}

#[async_trait]
impl ForViewingMarketData for MarketDataMock {
    async fn market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        self.requested
            .lock()
            .expect("test mutex must be usable")
            .push(ticker.to_string());
        Ok(MarketHistory {
            ticker: ticker.to_string(),
            currency: Some("USD".to_string()),
            exchange_timezone: None,
            daily_quotes: Vec::new(),
            dividends: Vec::new(),
            splits: Vec::new(),
        })
    }

    async fn index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        Ok(IndexHistory {
            ticker: ticker.to_string(),
            daily_prices: Vec::new(),
        })
    }

    async fn live_price(&self, ticker: &str) -> PortResult<LivePrice> {
        Ok(LivePrice {
            ticker: ticker.to_string(),
            price: 100.0,
            market_time: 0,
            currency: "USD".to_string(),
            exchange: "TEST".to_string(),
            regular_session: true,
            change: 0.0,
            change_percent: 0.0,
            day_volume: 0,
        })
    }
}

struct OptionsMock;
struct SimulationMock;
struct SynchronizationMock;
struct SavedStrategiesMock;
struct TrackedTickersMock;
struct SectorPerformanceMock;

#[async_trait]
impl ForResolvingUnderlyings for TrackedTickersMock {
    async fn resolve_underlying(&self, ticker: &str) -> PortResult<UnderlyingResolution> {
        match ticker {
            "bad ticker" => Err(PortError::InvalidRequest("invalid tracked ticker".into())),
            "MISSING" => Err(PortError::NotFound(
                "underlying MISSING was not found".into(),
            )),
            "UNAVAILABLE" => Err(PortError::Unavailable("Yahoo unavailable".into())),
            "INVALID" => Err(PortError::Unavailable(
                "Yahoo response was incompatible".into(),
            )),
            _ => Ok(UnderlyingResolution {
                ticker: ticker.trim().to_ascii_uppercase(),
                validated_at: Utc::now(),
                metadata: UnderlyingMetadata {
                    currency: Some("USD".into()),
                    exchange: Some("NMS".into()),
                    timezone: Some("America/New_York".into()),
                    instrument_type: Some("EQUITY".into()),
                },
            }),
        }
    }
}

fn market_ports(market_data: Arc<MarketDataMock>) -> http::MarketViewingPorts {
    http::MarketViewingPorts::new(market_data, Arc::new(SectorPerformanceMock))
}

#[async_trait]
impl ForViewingSectorPerformance for SectorPerformanceMock {
    async fn sector_performance(
        &self,
        period: hexagonal_backend::hexagon::domain::sector_performance::SectorPerformancePeriod,
        requested_at: DateTime<Utc>,
    ) -> PortResult<hexagonal_backend::hexagon::domain::sector_performance::SectorPerformanceView>
    {
        Ok(hexagonal_backend::hexagon::domain::sector_performance::SectorPerformanceView {
            as_of: requested_at.date_naive(),
            period,
            benchmark: hexagonal_backend::hexagon::domain::sector_performance::PerformanceState::Unavailable,
            sectors: Vec::new(),
        })
    }
}

#[async_trait]
impl ForManagingTrackedTickers for TrackedTickersMock {
    async fn list_tickers(&self, include_inactive: bool) -> PortResult<Vec<TrackedTicker>> {
        let tickers = vec![TrackedTicker {
            ticker: "QQQ".into(),
            source: hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::User,
            active: false,
            historical_prices: true,
            option_snapshots: false,
            resolution_state: UnderlyingResolutionState::Pending,
            validated_at: None,
            metadata: UnderlyingMetadata::default(),
        }];
        Ok(tickers
            .into_iter()
            .filter(|ticker| include_inactive || ticker.active)
            .collect())
    }

    async fn bootstrap_system_tickers(&self) -> PortResult<()> {
        Ok(())
    }

    async fn configure_ticker(
        &self,
        ticker: &str,
        _configuration: hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration,
    ) -> PortResult<()> {
        match ticker {
            "SPX" => Err(PortError::Conflict(
                "tracked ticker SPX is protected by the system".into(),
            )),
            "bad ticker" => Err(PortError::InvalidRequest("invalid tracked ticker".into())),
            _ => Ok(()),
        }
    }
}

#[async_trait]
impl ForManagingSavedStrategies for SavedStrategiesMock {
    async fn list_strategies(&self) -> PortResult<Vec<SavedStrategy>> {
        Ok(Vec::new())
    }

    async fn save_strategy(&self, command: SaveStrategy) -> PortResult<SavedStrategy> {
        Ok(SavedStrategy {
            id: 1,
            name: command.name,
            ticker: command.ticker,
            legs: command.legs,
            updated_at: Utc::now(),
        })
    }

    async fn delete_strategy(&self, id: i64) -> PortResult<()> {
        if id <= 0 {
            return Err(PortError::InvalidRequest("invalid id".to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl ForSynchronizingMarketData for SynchronizationMock {
    async fn synchronize_tracked_tickers(
        &self,
        _request: SynchronizeTrackedTickers,
    ) -> PortResult<TrackedTickersSynchronizationReport> {
        Ok(TrackedTickersSynchronizationReport {
            tickers: 0,
            items_obtained: 0,
            items_stored: 0,
            failures: Vec::new(),
        })
    }

    async fn synchronize_market_history(
        &self,
        _ticker: &str,
        _since: NaiveDate,
    ) -> PortResult<SynchronizationReport> {
        Ok(SynchronizationReport {
            items_obtained: 2,
            items_stored: 2,
        })
    }

    async fn synchronize_option_chain(
        &self,
        _ticker: &str,
        _market_close: DateTime<Utc>,
    ) -> PortResult<SynchronizationReport> {
        Ok(SynchronizationReport {
            items_obtained: 1,
            items_stored: 1,
        })
    }

    async fn synchronize_term_structure(&self, _ticker: &str) -> PortResult<SynchronizationReport> {
        Ok(SynchronizationReport {
            items_obtained: 1,
            items_stored: 1,
        })
    }

    async fn synchronize_volatility_index(
        &self,
        _ticker: &str,
    ) -> PortResult<SynchronizationReport> {
        Ok(SynchronizationReport {
            items_obtained: 1,
            items_stored: 1,
        })
    }

    async fn synchronize_yield_curves(&self, year: i32) -> PortResult<SynchronizationReport> {
        if year < 1900 {
            return Err(PortError::InvalidRequest("invalid year".to_string()));
        }
        Ok(SynchronizationReport {
            items_obtained: 1,
            items_stored: 1,
        })
    }
}
#[derive(Default)]
struct PortfoliosMock {
    created: Mutex<Vec<String>>,
}

#[async_trait]
impl ForManagingPortfolios for PortfoliosMock {
    async fn create_portfolio(&self, command: CreatePortfolio) -> PortResult<()> {
        if command.id == "conflict" {
            return Err(PortError::Conflict("portfolio already exists".to_string()));
        }
        self.created
            .lock()
            .expect("test mutex must be usable")
            .push(command.id);
        Ok(())
    }

    async fn portfolio(&self, portfolio_id: &str) -> PortResult<Portfolio> {
        Portfolio::new(portfolio_id, "Test", Currency::eur())
            .map_err(|error| PortError::InvalidRequest(error.to_string()))
    }

    async fn record_cash_movement(
        &self,
        _portfolio_id: &str,
        _movement: CashMovement,
    ) -> PortResult<()> {
        Ok(())
    }

    async fn record_option_trade(&self, _portfolio_id: &str, _trade: Trade) -> PortResult<()> {
        Ok(())
    }

    async fn record_currency_exchange(
        &self,
        _portfolio_id: &str,
        _exchange: CurrencyExchange,
    ) -> PortResult<()> {
        Ok(())
    }

    async fn check_balance(&self, _portfolio_id: &str) -> PortResult<BTreeMap<String, Decimal>> {
        Ok(BTreeMap::new())
    }

    async fn list_positions(&self, _portfolio_id: &str) -> PortResult<Vec<Position>> {
        Ok(Vec::new())
    }

    async fn list_transactions(&self, _portfolio_id: &str) -> PortResult<Vec<PortfolioEvent>> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl ForSimulatingStrategies for SimulationMock {
    async fn build_scenario_grid(&self, _request: ScenarioGridRequest) -> PortResult<ScenarioGrid> {
        Err(PortError::InvalidRequest("invalid mock grid".to_string()))
    }

    async fn simulate_strategy(&self, _request: SimulationRequest) -> PortResult<SimulationResult> {
        Err(PortError::Unavailable("unused".to_string()))
    }

    async fn simulate_scenario(
        &self,
        _command: SimulateScenario,
    ) -> PortResult<SimulationScenario> {
        Err(PortError::Unavailable("unused".to_string()))
    }
}

#[async_trait]
impl ForAnalyzingOptions for OptionsMock {
    async fn option_chain(&self, _ticker: &str) -> PortResult<Snapshot> {
        Err(PortError::NotFound("chain unavailable".to_string()))
    }

    async fn term_structure(&self, _ticker: &str) -> PortResult<TermStructure> {
        Err(PortError::Unavailable("unused".to_string()))
    }

    async fn volatility_surface(&self, _ticker: &str) -> PortResult<VolatilitySurface> {
        Err(PortError::Unavailable("unused".to_string()))
    }

    async fn volatility_skew(
        &self,
        _ticker: &str,
        _expiration: NaiveDate,
    ) -> PortResult<VolatilitySkew> {
        Err(PortError::Unavailable("unused".to_string()))
    }

    async fn greeks(&self, _request: GreeksRequest) -> PortResult<Greeks> {
        Err(PortError::Unavailable("unused".to_string()))
    }
}

#[tokio::test]
async fn http_adapter_drives_a_mock_application() {
    let market = Arc::new(MarketDataMock::default());
    let app = http::router(
        market_ports(market.clone()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/market-data/SPY/history")
                .body(Body::empty())
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 200);
    assert_eq!(
        *market.requested.lock().expect("test mutex must be usable"),
        vec!["SPY"]
    );
}

#[tokio::test]
async fn http_adapter_translates_application_errors() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/options/SPY/chain")
                .body(Body::empty())
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn http_adapter_drives_the_simulation_port_and_maps_validation_errors() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );
    let body = serde_json::json!({
        "spot": 0.0,
        "range_fraction": 0.2,
        "spot_count": 5,
        "valuation_dates": ["2026-01-01"],
        "volatility_shifts": [0.0],
        "required_spots": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/simulation/grid")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn http_adapter_drives_the_complete_portfolio_port() {
    let portfolios = Arc::new(PortfoliosMock::default());
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        portfolios.clone(),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );
    let body = serde_json::json!({
        "id": "main",
        "name": "Principal",
        "base_currency": "EUR"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/portfolios")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 201);
    assert_eq!(
        *portfolios
            .created
            .lock()
            .expect("test mutex must be usable"),
        vec!["main"]
    );
}

#[tokio::test]
async fn http_adapter_drives_the_synchronization_port() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let term_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/synchronization/term-structure/SPY")
                .body(Body::empty())
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");
    assert_eq!(term_response.status(), 200);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/synchronization/yield-curves/1800")
                .body(Body::empty())
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn http_adapter_drives_saved_strategy_management() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );
    let body = serde_json::json!({
        "name": "Long call",
        "ticker": "SPY",
        "legs": [{
            "occ_symbol": "SPY260101C00100000",
            "side": "Buy",
            "quantity": 1,
            "entry_price": 5.0
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/saved-strategies")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn http_adapter_drives_tracked_ticker_management() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );
    let body = serde_json::json!({
        "active": true,
        "historical_prices": true,
        "option_snapshots": true
    });

    for path in ["/api/tracked-tickers/SPY", "/tracked-tickers/SPY"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("valid test request"),
            )
            .await
            .expect("router must respond");

        assert_eq!(response.status(), 204);
        assert!(response.headers().get("content-type").is_none());
        assert!(
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty()
        );
    }
}

#[tokio::test]
async fn tracked_ticker_http_contract_lists_inactive_and_maps_protection_and_validation() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    for path in ["/api/tracked-tickers", "/tracked-tickers"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!([])
        );
    }

    for path in [
        "/api/tracked-tickers?include_inactive=true",
        "/tracked-tickers?include_inactive=true",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!([{
                "ticker": "QQQ",
                "source": "user",
                "active": false,
                "historical_prices": true,
                "option_snapshots": false
                ,"resolution_state": "pending",
                "validated_at": null,
                "metadata": {
                    "currency": null,
                    "exchange": null,
                    "timezone": null,
                    "instrument_type": null
                }
            }])
        );
    }

    for (path, expected) in [
        ("/api/tracked-tickers/SPX", StatusCode::CONFLICT),
        ("/tracked-tickers/bad%20ticker", StatusCode::BAD_REQUEST),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"active":false,"historical_prices":false,"option_snapshots":false}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

#[tokio::test]
async fn exact_underlying_resolution_has_canonical_success_and_public_errors_only() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/underlyings/resolve?ticker=MSFT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["ticker"], "MSFT");
    assert_eq!(json["metadata"]["instrument_type"], "EQUITY");

    for (ticker, status) in [
        ("bad%20ticker", StatusCode::BAD_REQUEST),
        ("MISSING", StatusCode::NOT_FOUND),
        ("UNAVAILABLE", StatusCode::SERVICE_UNAVAILABLE),
        ("INVALID", StatusCode::SERVICE_UNAVAILABLE),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/underlyings/resolve?ticker={ticker}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }

    let no_alias = app
        .oneshot(
            Request::builder()
                .uri("/underlyings/resolve?ticker=MSFT")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_alias.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sector_endpoint_accepts_supported_periods_and_returns_json() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    for period in ["1w", "2w", "1m"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/market/sectors?period={period}"))
                    .body(Body::empty())
                    .expect("valid test request"),
            )
            .await
            .expect("router must respond");
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["period"], period);
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/market/sectors?period=30d")
                .body(Body::empty())
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");
    assert_eq!(response.status(), 400);
    assert_eq!(response.headers()["content-type"], "application/json");
}

#[tokio::test]
async fn canonical_route_and_alias_have_identical_http_response() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let canonical = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/market-data/SPY/history")
                .body(Body::empty())
                .expect("valid canonical request"),
        )
        .await
        .expect("canonical route must respond");
    let alias = app
        .oneshot(
            Request::builder()
                .uri("/market-data/SPY/history")
                .body(Body::empty())
                .expect("valid alias request"),
        )
        .await
        .expect("alias must respond");

    assert_eq!(canonical.status(), alias.status());
    assert_eq!(
        canonical.headers().get("content-type"),
        alias.headers().get("content-type")
    );
    let canonical_body = axum::body::to_bytes(canonical.into_body(), usize::MAX)
        .await
        .expect("canonical body must be readable");
    let alias_body = axum::body::to_bytes(alias.into_body(), usize::MAX)
        .await
        .expect("alias body must be readable");
    assert_eq!(canonical_body, alias_body);
}

#[tokio::test]
async fn canonical_extractor_errors_are_json_without_changing_alias_errors() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let malformed = Request::builder()
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("valid malformed request");
    let canonical = app
        .clone()
        .oneshot(clone_request(&malformed, "/api/portfolios"))
        .await
        .expect("canonical route must respond");
    let alias = app
        .clone()
        .oneshot(clone_request(&malformed, "/portfolios"))
        .await
        .expect("alias must respond");
    assert_eq!(canonical.status(), alias.status());
    assert_eq!(canonical.headers()["content-type"], "application/json");
    assert_eq!(alias.headers()["content-type"], "text/plain; charset=utf-8");
    let canonical_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(canonical.into_body(), usize::MAX)
            .await
            .expect("canonical body must be readable"),
    )
    .expect("canonical error must be JSON");
    assert!(canonical_json["error"].is_string());

    let invalid_query = app
        .oneshot(
            Request::builder()
                .uri("/api/market/sectors")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("query rejection must respond");
    assert_eq!(invalid_query.status(), 400);
    assert_eq!(invalid_query.headers()["content-type"], "application/json");
}

fn clone_request(request: &Request<Body>, uri: &str) -> Request<Body> {
    Request::builder()
        .method(request.method().clone())
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("test request must be valid")
}

#[tokio::test]
async fn port_errors_keep_status_and_json_envelope_on_canonical_and_alias_routes() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );
    let cases = [
        (
            "GET",
            "/api/options/SPY/chain",
            "/options/SPY/chain",
            None,
            404,
        ),
        (
            "GET",
            "/api/options/SPY/term-structure",
            "/options/SPY/term-structure",
            None,
            503,
        ),
        (
            "POST",
            "/api/strategy-simulation/grid",
            "/simulation/grid",
            Some(serde_json::json!({
                "spot": 0.0, "range_fraction": 0.2, "spot_count": 5,
                "valuation_dates": ["2026-01-01"], "volatility_shifts": [0.0], "required_spots": []
            })),
            400,
        ),
        (
            "POST",
            "/api/portfolios",
            "/portfolios",
            Some(serde_json::json!({
                "id": "conflict", "name": "Conflict", "base_currency": "EUR"
            })),
            409,
        ),
    ];
    for (method, canonical_uri, alias_uri, body, status) in cases {
        let payload = body.map(|value| value.to_string()).unwrap_or_default();
        let request = |uri: &str| {
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .expect("test request must be valid")
        };
        let canonical = app
            .clone()
            .oneshot(request(canonical_uri))
            .await
            .expect("canonical response");
        let alias = app
            .clone()
            .oneshot(request(alias_uri))
            .await
            .expect("alias response");
        assert_eq!(canonical.status(), status);
        assert_eq!(alias.status(), status);
        assert_eq!(canonical.headers()["content-type"], "application/json");
        assert_eq!(alias.headers()["content-type"], "application/json");
        let canonical_body = axum::body::to_bytes(canonical.into_body(), usize::MAX)
            .await
            .expect("body");
        let alias_body = axum::body::to_bytes(alias.into_body(), usize::MAX)
            .await
            .expect("body");
        assert_eq!(canonical_body, alias_body);
    }
}

#[tokio::test]
async fn canonical_method_not_allowed_keeps_allow_header_and_returns_json() {
    let app = http::router(
        market_ports(Arc::new(MarketDataMock::default())),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        http::TrackedTickerPorts::new(Arc::new(TrackedTickersMock), Arc::new(TrackedTickersMock)),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/portfolios")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("canonical route must respond");

    assert_eq!(response.status(), 405);
    assert_eq!(response.headers()["allow"], "POST");
    assert_eq!(response.headers()["content-type"], "application/json");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("error body must be readable");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("error body must be JSON");
    assert_eq!(json, serde_json::json!({ "error": "Method Not Allowed" }));
}
