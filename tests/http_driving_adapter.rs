use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{body::Body, http::Request};
use chrono::{DateTime, NaiveDate, Utc};
use polars_options::{
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
            tracked_ticker::TrackedTicker,
            volatility::TermStructure,
            volatility_surface::{VolatilitySkew, VolatilitySurface},
        },
        driving_ports::{
            for_analyzing_options::{ForAnalyzingOptions, GreeksRequest},
            for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
            for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
            for_managing_tracked_tickers::ForManagingTrackedTickers,
            for_simulating_strategies::{
                ForSimulatingStrategies, ScenarioGridRequest, SimulateScenario,
            },
            for_synchronizing_market_data::{
                ForSynchronizingMarketData, SynchronizationReport, SynchronizeTrackedTickers,
                TrackedTickersSynchronizationReport,
            },
            for_viewing_market_data::ForViewingMarketData,
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

#[async_trait]
impl ForManagingTrackedTickers for TrackedTickersMock {
    async fn list_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        Ok(Vec::new())
    }

    async fn configure_ticker(&self, _ticker: TrackedTicker) -> PortResult<()> {
        Ok(())
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
        market.clone(),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
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
        Arc::new(MarketDataMock::default()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
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
        Arc::new(MarketDataMock::default()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
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
        Arc::new(MarketDataMock::default()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        portfolios.clone(),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
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
        Arc::new(MarketDataMock::default()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
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
        Arc::new(MarketDataMock::default()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
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
        Arc::new(MarketDataMock::default()),
        Arc::new(OptionsMock),
        Arc::new(SimulationMock),
        Arc::new(PortfoliosMock::default()),
        Arc::new(SynchronizationMock),
        Arc::new(SavedStrategiesMock),
        Arc::new(TrackedTickersMock),
    );
    let body = serde_json::json!({
        "active": true,
        "historical_prices": true,
        "option_snapshots": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/tracked-tickers/SPY")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("valid test request"),
        )
        .await
        .expect("router must respond");

    assert_eq!(response.status(), 204);
}
