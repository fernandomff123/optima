//! HTTP driving adapter.
//!
//! Axum types stop at this boundary. Handlers validate transport input, invoke
//! driving ports, and translate application errors to HTTP responses.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::hexagon::{
    PortError,
    driving_ports::{
        for_analyzing_options::{ForAnalyzingOptions, GreeksRequest},
        for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
        for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
        for_managing_tracked_tickers::ForManagingTrackedTickers,
        for_refreshing_market_data::ForRefreshingMarketData,
        for_simulating_strategies::{ForSimulatingStrategies, ScenarioGridRequest},
        for_synchronizing_market_data::ForSynchronizingMarketData,
        for_synchronizing_market_data::SynchronizeTrackedTickers,
        for_viewing_market_data::ForViewingMarketData,
        for_viewing_sector_performance::ForViewingSectorPerformance,
    },
};

pub mod legacy_asset_views;
pub mod legacy_market_views;
pub mod legacy_portfolio_views;
pub mod legacy_simulation_views;
pub mod sector_performance_views;

#[derive(Clone)]
struct HttpState {
    market_data: Arc<dyn ForViewingMarketData>,
    options: Arc<dyn ForAnalyzingOptions>,
    simulation: Arc<dyn ForSimulatingStrategies>,
    portfolios: Arc<dyn ForManagingPortfolios>,
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    saved_strategies: Arc<dyn ForManagingSavedStrategies>,
    tracked_tickers: Arc<dyn ForManagingTrackedTickers>,
    sector_performance: Arc<dyn ForViewingSectorPerformance>,
    data_refresh: Option<Arc<dyn ForRefreshingMarketData>>,
    manual_refresh_start: Arc<tokio::sync::Mutex<()>>,
}

pub struct MarketViewingPorts {
    market_data: Arc<dyn ForViewingMarketData>,
    sector_performance: Arc<dyn ForViewingSectorPerformance>,
}

pub struct SynchronizationPorts {
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    data_refresh: Option<Arc<dyn ForRefreshingMarketData>>,
}

impl SynchronizationPorts {
    pub fn new(
        synchronization: Arc<dyn ForSynchronizingMarketData>,
        data_refresh: Arc<dyn ForRefreshingMarketData>,
    ) -> Self {
        Self {
            synchronization,
            data_refresh: Some(data_refresh),
        }
    }
}

impl MarketViewingPorts {
    pub fn new(
        market_data: Arc<dyn ForViewingMarketData>,
        sector_performance: Arc<dyn ForViewingSectorPerformance>,
    ) -> Self {
        Self {
            market_data,
            sector_performance,
        }
    }
}

/// Builds an HTTP adapter around application-provided interfaces.
pub fn router(
    market_viewing: MarketViewingPorts,
    options: Arc<dyn ForAnalyzingOptions>,
    simulation: Arc<dyn ForSimulatingStrategies>,
    portfolios: Arc<dyn ForManagingPortfolios>,
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    saved_strategies: Arc<dyn ForManagingSavedStrategies>,
    tracked_tickers: Arc<dyn ForManagingTrackedTickers>,
) -> Router {
    router_with_data_refresh(
        market_viewing,
        options,
        simulation,
        portfolios,
        SynchronizationPorts {
            synchronization,
            data_refresh: None,
        },
        saved_strategies,
        tracked_tickers,
    )
}

pub fn router_with_data_refresh(
    market_viewing: MarketViewingPorts,
    options: Arc<dyn ForAnalyzingOptions>,
    simulation: Arc<dyn ForSimulatingStrategies>,
    portfolios: Arc<dyn ForManagingPortfolios>,
    synchronization: SynchronizationPorts,
    saved_strategies: Arc<dyn ForManagingSavedStrategies>,
    tracked_tickers: Arc<dyn ForManagingTrackedTickers>,
) -> Router {
    Router::new()
        .route("/market-data/{ticker}/history", get(market_history))
        .route("/market-data/{ticker}/live-price", get(live_price))
        .route("/options/{ticker}/chain", get(option_chain))
        .route("/options/{ticker}/term-structure", get(term_structure))
        .route("/options/{ticker}/surface", get(volatility_surface))
        .route("/options/{ticker}/skew/{expiration}", get(volatility_skew))
        .route(
            "/options/{ticker}/contracts/{occ_symbol}/greeks",
            get(greeks),
        )
        .route("/simulation/grid", post(build_scenario_grid))
        .route("/simulation", post(simulate_strategy))
        .route("/portfolios", post(create_portfolio))
        .route(
            "/portfolios/{id}/cash-movements",
            post(record_cash_movement),
        )
        .route("/portfolios/{id}/option-trades", post(record_option_trade))
        .route(
            "/portfolios/{id}/currency-exchanges",
            post(record_currency_exchange),
        )
        .route("/portfolios/{id}/balance", get(check_balance))
        .route("/portfolios/{id}/positions", get(list_positions))
        .route("/portfolios/{id}/transactions", get(list_transactions))
        .route(
            "/synchronization/market-history/{ticker}",
            post(synchronize_market_history),
        )
        .route(
            "/synchronization/tracked-tickers",
            post(synchronize_tracked_tickers),
        )
        .route(
            "/synchronization/option-chain/{ticker}",
            post(synchronize_option_chain),
        )
        .route(
            "/synchronization/term-structure/{ticker}",
            post(synchronize_term_structure),
        )
        .route(
            "/synchronization/volatility-index/{ticker}",
            post(synchronize_volatility_index),
        )
        .route(
            "/synchronization/yield-curves/{year}",
            post(synchronize_yield_curves),
        )
        .route(
            "/saved-strategies",
            get(list_saved_strategies).post(save_strategy),
        )
        .route(
            "/saved-strategies/{id}",
            axum::routing::delete(delete_strategy),
        )
        .route("/tracked-tickers", get(list_tracked_tickers))
        .route("/api/market/sectors", get(view_sector_performance))
        .route("/api/data-refresh/status", get(data_refresh_status))
        .route("/api/data-refresh", post(request_data_refresh))
        .route(
            "/tracked-tickers/{ticker}",
            axum::routing::put(configure_tracked_ticker),
        )
        .with_state(HttpState {
            market_data: market_viewing.market_data,
            options,
            simulation,
            portfolios,
            synchronization: synchronization.synchronization,
            saved_strategies,
            tracked_tickers,
            sector_performance: market_viewing.sector_performance,
            data_refresh: synchronization.data_refresh,
            manual_refresh_start: Arc::new(tokio::sync::Mutex::new(())),
        })
}

async fn data_refresh_status(
    State(state): State<HttpState>,
) -> Result<Json<api_models::DataRefreshStatusResponse>, HttpError> {
    refresh_port(&state)?
        .data_refresh_status(20)
        .await
        .map(|status| Json(map_refresh_status(status)))
        .map_err(HttpError)
}

async fn request_data_refresh(
    State(state): State<HttpState>,
) -> Result<(StatusCode, Json<api_models::DataRefreshRequestResponse>), HttpError> {
    let _start_guard = state.manual_refresh_start.lock().await;
    let refresh = refresh_port(&state)?;
    if refresh
        .eligible_data_refresh_session(chrono::Utc::now())
        .map_err(HttpError)?
        .is_none()
    {
        return Ok((
            StatusCode::CONFLICT,
            Json(api_models::DataRefreshRequestResponse {
                result: api_models::DataRefreshRequestState::NoEligibleSession,
                run: None,
                message: "Não existe uma sessão de mercado concluída elegível".to_string(),
            }),
        ));
    }
    let status = state
        .data_refresh
        .as_ref()
        .ok_or_else(|| {
            HttpError(PortError::Unavailable(
                "data refresh is not configured".to_string(),
            ))
        })?
        .data_refresh_status(1)
        .await
        .map_err(HttpError)?;
    if status.running {
        return Ok((
            StatusCode::CONFLICT,
            Json(api_models::DataRefreshRequestResponse {
                result: api_models::DataRefreshRequestState::AlreadyRunning,
                run: status.latest.map(map_refresh_run),
                message: "Já existe uma atualização em curso".to_string(),
            }),
        ));
    }
    let application = refresh.clone();
    tokio::spawn(async move {
        if let Err(error) = application
            .refresh_market_data(
                crate::hexagon::domain::data_refresh::DataRefreshOrigin::Manual,
                chrono::Utc::now(),
            )
            .await
        {
            eprintln!("Falha ao iniciar atualização manual: {error}");
        }
    });
    // Keep the boundary-level start decision serialized until the application
    // has persisted the run. The long-running refresh remains detached.
    for _ in 0..100 {
        let status = refresh.data_refresh_status(1).await.map_err(HttpError)?;
        if status.running {
            break;
        }
        tokio::task::yield_now().await;
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(api_models::DataRefreshRequestResponse {
            result: api_models::DataRefreshRequestState::Started,
            run: None,
            message: "Atualização manual iniciada".to_string(),
        }),
    ))
}

fn refresh_port(state: &HttpState) -> Result<&Arc<dyn ForRefreshingMarketData>, HttpError> {
    state.data_refresh.as_ref().ok_or_else(|| {
        HttpError(PortError::Unavailable(
            "data refresh is not configured".to_string(),
        ))
    })
}

fn map_refresh_status(
    status: crate::hexagon::driving_ports::for_refreshing_market_data::DataRefreshStatus,
) -> api_models::DataRefreshStatusResponse {
    api_models::DataRefreshStatusResponse {
        running: status.running,
        latest: status.latest.map(map_refresh_run),
        recent: status.recent.into_iter().map(map_refresh_run).collect(),
    }
}
fn map_refresh_run(
    run: crate::hexagon::domain::data_refresh::DataRefreshRun,
) -> api_models::DataRefreshRun {
    use crate::hexagon::domain::data_refresh::{DataRefreshOrigin as O, DataRefreshState as S};
    api_models::DataRefreshRun {
        id: run.id,
        origin: match run.origin {
            O::Startup => api_models::DataRefreshOrigin::Startup,
            O::Scheduled => api_models::DataRefreshOrigin::Scheduled,
            O::Retry => api_models::DataRefreshOrigin::Retry,
            O::Manual => api_models::DataRefreshOrigin::Manual,
        },
        state: match run.state {
            S::Running => api_models::DataRefreshState::Running,
            S::Completed => api_models::DataRefreshState::Completed,
            S::Partial => api_models::DataRefreshState::Partial,
            S::Failed => api_models::DataRefreshState::Failed,
        },
        started_at: run.started_at,
        finished_at: run.finished_at,
        target_session: run.target_session,
        items_obtained: run.items_obtained,
        items_persisted: run.items_persisted,
        failure_count: run.failure_count,
        next_attempt_at: run.next_attempt_at,
        summary: run.summary,
        failures: run
            .failures
            .into_iter()
            .map(|failure| api_models::DataRefreshFailure {
                ticker: failure.ticker,
                operation: failure.operation,
                error: failure.error,
            })
            .collect(),
    }
}

#[derive(Deserialize)]
struct SectorPerformanceQuery {
    period: String,
}

async fn view_sector_performance(
    State(state): State<HttpState>,
    Query(query): Query<SectorPerformanceQuery>,
) -> Result<Json<api_models::MarketSectorPerformanceResponse>, HttpError> {
    use crate::hexagon::domain::sector_performance::SectorPerformancePeriod;

    let period = match query.period.as_str() {
        "1w" => SectorPerformancePeriod::OneWeek,
        "2w" => SectorPerformancePeriod::TwoWeeks,
        "1m" => SectorPerformancePeriod::OneMonth,
        _ => {
            return Err(HttpError(PortError::InvalidRequest(
                "period must be one of: 1w, 2w, 1m".to_string(),
            )));
        }
    };
    state
        .sector_performance
        .sector_performance(period, chrono::Utc::now())
        .await
        .map(sector_performance_views::response)
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_tracked_tickers(
    State(state): State<HttpState>,
    Json(request): Json<SynchronizeTrackedTickers>,
) -> Result<
    Json<
        crate::hexagon::driving_ports::for_synchronizing_market_data::TrackedTickersSynchronizationReport,
    >,
    HttpError,
>{
    state
        .synchronization
        .synchronize_tracked_tickers(request)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn list_tracked_tickers(
    State(state): State<HttpState>,
) -> Result<Json<Vec<crate::hexagon::domain::tracked_ticker::TrackedTicker>>, HttpError> {
    state
        .tracked_tickers
        .list_active_tickers()
        .await
        .map(Json)
        .map_err(HttpError)
}

#[derive(Deserialize)]
struct ConfigureTrackedTickerBody {
    active: bool,
    historical_prices: bool,
    option_snapshots: bool,
}

async fn configure_tracked_ticker(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Json(body): Json<ConfigureTrackedTickerBody>,
) -> Result<StatusCode, HttpError> {
    state
        .tracked_tickers
        .configure_ticker(crate::hexagon::domain::tracked_ticker::TrackedTicker {
            ticker,
            active: body.active,
            historical_prices: body.historical_prices,
            option_snapshots: body.option_snapshots,
        })
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn list_saved_strategies(
    State(state): State<HttpState>,
) -> Result<Json<Vec<crate::hexagon::domain::saved_strategy::SavedStrategy>>, HttpError> {
    state
        .saved_strategies
        .list_strategies()
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn save_strategy(
    State(state): State<HttpState>,
    Json(command): Json<SaveStrategy>,
) -> Result<Json<crate::hexagon::domain::saved_strategy::SavedStrategy>, HttpError> {
    state
        .saved_strategies
        .save_strategy(command)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn delete_strategy(
    State(state): State<HttpState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, HttpError> {
    state
        .saved_strategies
        .delete_strategy(id)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

#[derive(Deserialize)]
struct MarketHistorySynchronizationBody {
    since: NaiveDate,
}

async fn synchronize_market_history(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Json(body): Json<MarketHistorySynchronizationBody>,
) -> Result<
    Json<crate::hexagon::driving_ports::for_synchronizing_market_data::SynchronizationReport>,
    HttpError,
> {
    state
        .synchronization
        .synchronize_market_history(&ticker, body.since)
        .await
        .map(Json)
        .map_err(HttpError)
}

#[derive(Deserialize)]
struct OptionChainSynchronizationBody {
    market_close: chrono::DateTime<chrono::Utc>,
}

async fn synchronize_option_chain(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Json(body): Json<OptionChainSynchronizationBody>,
) -> Result<
    Json<crate::hexagon::driving_ports::for_synchronizing_market_data::SynchronizationReport>,
    HttpError,
> {
    state
        .synchronization
        .synchronize_option_chain(&ticker, body.market_close)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_term_structure(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<
    Json<crate::hexagon::driving_ports::for_synchronizing_market_data::SynchronizationReport>,
    HttpError,
> {
    state
        .synchronization
        .synchronize_term_structure(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_volatility_index(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<
    Json<crate::hexagon::driving_ports::for_synchronizing_market_data::SynchronizationReport>,
    HttpError,
> {
    state
        .synchronization
        .synchronize_volatility_index(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_yield_curves(
    State(state): State<HttpState>,
    Path(year): Path<i32>,
) -> Result<
    Json<crate::hexagon::driving_ports::for_synchronizing_market_data::SynchronizationReport>,
    HttpError,
> {
    state
        .synchronization
        .synchronize_yield_curves(year)
        .await
        .map(Json)
        .map_err(HttpError)
}

#[derive(Deserialize)]
struct CreatePortfolioBody {
    id: String,
    name: String,
    base_currency: String,
}

async fn create_portfolio(
    State(state): State<HttpState>,
    Json(body): Json<CreatePortfolioBody>,
) -> Result<StatusCode, HttpError> {
    let base_currency = crate::hexagon::domain::portfolio::Currency::new(&body.base_currency)
        .map_err(|error| HttpError(PortError::InvalidRequest(error.to_string())))?;
    state
        .portfolios
        .create_portfolio(CreatePortfolio {
            id: body.id,
            name: body.name,
            base_currency,
        })
        .await
        .map(|()| StatusCode::CREATED)
        .map_err(HttpError)
}

async fn record_cash_movement(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    Json(movement): Json<crate::hexagon::domain::portfolio::CashMovement>,
) -> Result<StatusCode, HttpError> {
    state
        .portfolios
        .record_cash_movement(&id, movement)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn record_option_trade(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    Json(trade): Json<crate::hexagon::domain::portfolio::Trade>,
) -> Result<StatusCode, HttpError> {
    state
        .portfolios
        .record_option_trade(&id, trade)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn record_currency_exchange(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    Json(exchange): Json<crate::hexagon::domain::portfolio::CurrencyExchange>,
) -> Result<StatusCode, HttpError> {
    state
        .portfolios
        .record_currency_exchange(&id, exchange)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn check_balance(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<std::collections::BTreeMap<String, rust_decimal::Decimal>>, HttpError> {
    state
        .portfolios
        .check_balance(&id)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn list_positions(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::hexagon::domain::portfolio::Position>>, HttpError> {
    state
        .portfolios
        .list_positions(&id)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn list_transactions(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::hexagon::domain::portfolio::PortfolioEvent>>, HttpError> {
    state
        .portfolios
        .list_transactions(&id)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn build_scenario_grid(
    State(state): State<HttpState>,
    Json(request): Json<ScenarioGridRequest>,
) -> Result<Json<crate::hexagon::domain::simulation::ScenarioGrid>, HttpError> {
    state
        .simulation
        .build_scenario_grid(request)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn simulate_strategy(
    State(state): State<HttpState>,
    Json(request): Json<crate::hexagon::domain::simulation::SimulationRequest>,
) -> Result<Json<crate::hexagon::domain::simulation::SimulationResult>, HttpError> {
    state
        .simulation
        .simulate_strategy(request)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn market_history(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<crate::hexagon::domain::market_history::MarketHistory>, HttpError> {
    state
        .market_data
        .market_history(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn live_price(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<crate::hexagon::domain::live_price::LivePrice>, HttpError> {
    state
        .market_data
        .live_price(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn option_chain(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<crate::hexagon::domain::options::Snapshot>, HttpError> {
    state
        .options
        .option_chain(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn term_structure(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<crate::hexagon::domain::volatility::TermStructure>, HttpError> {
    state
        .options
        .term_structure(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn volatility_surface(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<crate::hexagon::domain::volatility_surface::VolatilitySurface>, HttpError> {
    state
        .options
        .volatility_surface(&ticker)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn volatility_skew(
    State(state): State<HttpState>,
    Path((ticker, expiration)): Path<(String, String)>,
) -> Result<Json<crate::hexagon::domain::volatility_surface::VolatilitySkew>, HttpError> {
    let expiration = NaiveDate::parse_from_str(&expiration, "%Y-%m-%d")
        .map_err(|_| HttpError(PortError::InvalidRequest("invalid expiration".into())))?;
    state
        .options
        .volatility_skew(&ticker, expiration)
        .await
        .map(Json)
        .map_err(HttpError)
}

async fn greeks(
    State(state): State<HttpState>,
    Path((ticker, occ_symbol)): Path<(String, String)>,
) -> Result<Json<crate::hexagon::domain::simulation::Greeks>, HttpError> {
    state
        .options
        .greeks(GreeksRequest { ticker, occ_symbol })
        .await
        .map(Json)
        .map_err(HttpError)
}

#[derive(Debug)]
struct HttpError(PortError);

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            PortError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            PortError::NotFound(_) => StatusCode::NOT_FOUND,
            PortError::Conflict(_) => StatusCode::CONFLICT,
            PortError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = ErrorBody {
            error: self.0.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
