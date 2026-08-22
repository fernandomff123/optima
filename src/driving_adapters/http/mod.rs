//! HTTP driving adapter.
//!
//! Axum types stop at this boundary. Handlers validate transport input, invoke
//! driving ports, and translate application errors to HTTP responses.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use chrono::{NaiveDate, Utc};

use crate::hexagon::{
    PortError,
    driving_ports::{
        for_analyzing_options::{ForAnalyzingOptions, GreeksRequest},
        for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
        for_managing_saved_strategies::ForManagingSavedStrategies,
        for_managing_tracked_tickers::ForManagingTrackedTickers,
        for_refreshing_market_data::ForRefreshingMarketData,
        for_resolving_underlyings::ForResolvingUnderlyings,
        for_simulating_strategies::ForSimulatingStrategies,
        for_synchronizing_market_data::ForSynchronizingMarketData,
        for_synchronizing_market_data::SynchronizeTrackedTickers,
        for_viewing_gamma_exposure::{ForViewingGammaExposure, GammaExposureRequest},
        for_viewing_market_data::ForViewingMarketData,
        for_viewing_sector_performance::ForViewingSectorPerformance,
    },
};

mod canonical_models;
pub mod legacy_asset_views;
pub mod legacy_market_views;
pub mod legacy_portfolio_views;
pub mod legacy_server;
pub mod legacy_simulation_views;
pub mod sector_performance_views;

/// Canonical route, temporary alias, method and shared handler.
pub const CANONICAL_ALIASES: &[(&str, &str, &str, &str)] = &[
    (
        "/api/market-data/{ticker}/history",
        "/market-data/{ticker}/history",
        "GET",
        "market_history",
    ),
    (
        "/api/market-data/{ticker}/live-price",
        "/market-data/{ticker}/live-price",
        "GET",
        "live_price",
    ),
    (
        "/api/options/{ticker}/chain",
        "/options/{ticker}/chain",
        "GET",
        "option_chain",
    ),
    (
        "/api/options/{ticker}/term-structure",
        "/options/{ticker}/term-structure",
        "GET",
        "term_structure",
    ),
    (
        "/api/options/{ticker}/surface",
        "/options/{ticker}/surface",
        "GET",
        "volatility_surface",
    ),
    (
        "/api/options/{ticker}/skew/{expiration}",
        "/options/{ticker}/skew/{expiration}",
        "GET",
        "volatility_skew",
    ),
    (
        "/api/options/{ticker}/contracts/{occ_symbol}/greeks",
        "/options/{ticker}/contracts/{occ_symbol}/greeks",
        "GET",
        "greeks",
    ),
    (
        "/api/strategy-simulation/grid",
        "/simulation/grid",
        "POST",
        "build_scenario_grid",
    ),
    (
        "/api/strategy-simulation",
        "/simulation",
        "POST",
        "simulate_strategy",
    ),
    ("/api/portfolios", "/portfolios", "POST", "create_portfolio"),
    (
        "/api/portfolios/{id}/cash-movements",
        "/portfolios/{id}/cash-movements",
        "POST",
        "record_cash_movement",
    ),
    (
        "/api/portfolios/{id}/option-trades",
        "/portfolios/{id}/option-trades",
        "POST",
        "record_option_trade",
    ),
    (
        "/api/portfolios/{id}/currency-exchanges",
        "/portfolios/{id}/currency-exchanges",
        "POST",
        "record_currency_exchange",
    ),
    (
        "/api/portfolios/{id}/balance",
        "/portfolios/{id}/balance",
        "GET",
        "check_balance",
    ),
    (
        "/api/portfolios/{id}/positions",
        "/portfolios/{id}/positions",
        "GET",
        "list_positions",
    ),
    (
        "/api/portfolios/{id}/transactions",
        "/portfolios/{id}/transactions",
        "GET",
        "list_transactions",
    ),
    (
        "/api/saved-strategies",
        "/saved-strategies",
        "GET",
        "list_saved_strategies",
    ),
    (
        "/api/saved-strategies",
        "/saved-strategies",
        "POST",
        "save_strategy",
    ),
    (
        "/api/saved-strategies/{id}",
        "/saved-strategies/{id}",
        "DELETE",
        "delete_strategy",
    ),
    (
        "/api/tracked-tickers",
        "/tracked-tickers",
        "GET",
        "list_tracked_tickers",
    ),
    (
        "/api/tracked-tickers/{ticker}",
        "/tracked-tickers/{ticker}",
        "PUT",
        "configure_tracked_ticker",
    ),
];

pub const EXISTING_CANONICAL_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/options/gamma-exposure/{ticker}"),
    ("GET", "/api/market/sectors"),
    ("GET", "/api/data-refresh/status"),
    ("POST", "/api/data-refresh"),
    ("GET", "/api/assets/live"),
    ("GET", "/api/underlyings/resolve"),
];

pub const NON_CANONICAL_SYNCHRONIZATION_ALIASES: &[(&str, &str)] = &[
    ("POST", "/synchronization/market-history/{ticker}"),
    ("POST", "/synchronization/tracked-tickers"),
    ("POST", "/synchronization/option-chain/{ticker}"),
    ("POST", "/synchronization/term-structure/{ticker}"),
    ("POST", "/synchronization/volatility-index/{ticker}"),
    ("POST", "/synchronization/yield-curves/{year}"),
];

#[derive(Clone)]
struct HttpState {
    market_data: Arc<dyn ForViewingMarketData>,
    options: Arc<dyn ForAnalyzingOptions>,
    gamma_exposure: Arc<dyn ForViewingGammaExposure>,
    simulation: Arc<dyn ForSimulatingStrategies>,
    portfolios: Arc<dyn ForManagingPortfolios>,
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    saved_strategies: Arc<dyn ForManagingSavedStrategies>,
    tracked_tickers: Arc<dyn ForManagingTrackedTickers>,
    underlying_resolver: Arc<dyn ForResolvingUnderlyings>,
    sector_performance: Arc<dyn ForViewingSectorPerformance>,
    data_refresh: Option<Arc<dyn ForRefreshingMarketData>>,
}

pub struct MarketViewingPorts {
    market_data: Arc<dyn ForViewingMarketData>,
    sector_performance: Arc<dyn ForViewingSectorPerformance>,
}

pub struct OptionsViewingPorts {
    options: Arc<dyn ForAnalyzingOptions>,
    gamma_exposure: Arc<dyn ForViewingGammaExposure>,
}

impl OptionsViewingPorts {
    pub fn new(
        options: Arc<dyn ForAnalyzingOptions>,
        gamma_exposure: Arc<dyn ForViewingGammaExposure>,
    ) -> Self {
        Self {
            options,
            gamma_exposure,
        }
    }
}

pub struct SynchronizationPorts {
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    data_refresh: Option<Arc<dyn ForRefreshingMarketData>>,
}

pub struct TrackedTickerPorts {
    management: Arc<dyn ForManagingTrackedTickers>,
    resolution: Arc<dyn ForResolvingUnderlyings>,
}

impl TrackedTickerPorts {
    pub fn new(
        management: Arc<dyn ForManagingTrackedTickers>,
        resolution: Arc<dyn ForResolvingUnderlyings>,
    ) -> Self {
        Self {
            management,
            resolution,
        }
    }
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
    tracked_tickers: TrackedTickerPorts,
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
    tracked_tickers: TrackedTickerPorts,
) -> Router {
    router_with_data_refresh_and_gamma_exposure(
        market_viewing,
        OptionsViewingPorts::new(options, Arc::new(UnconfiguredGammaExposure)),
        simulation,
        portfolios,
        synchronization,
        saved_strategies,
        tracked_tickers,
    )
}

pub fn router_with_data_refresh_and_gamma_exposure(
    market_viewing: MarketViewingPorts,
    options_viewing: OptionsViewingPorts,
    simulation: Arc<dyn ForSimulatingStrategies>,
    portfolios: Arc<dyn ForManagingPortfolios>,
    synchronization: SynchronizationPorts,
    saved_strategies: Arc<dyn ForManagingSavedStrategies>,
    tracked_tickers: TrackedTickerPorts,
) -> Router {
    let canonical = Router::new()
        .route("/api/market-data/{ticker}/history", get(market_history))
        .route("/api/market-data/{ticker}/live-price", get(live_price))
        .route("/api/options/{ticker}/chain", get(option_chain))
        .route(
            "/api/options/gamma-exposure/{ticker}",
            get(view_gamma_exposure),
        )
        .route("/api/options/{ticker}/term-structure", get(term_structure))
        .route("/api/options/{ticker}/surface", get(volatility_surface))
        .route(
            "/api/options/{ticker}/skew/{expiration}",
            get(volatility_skew),
        )
        .route(
            "/api/options/{ticker}/contracts/{occ_symbol}/greeks",
            get(greeks),
        )
        .route("/api/strategy-simulation/grid", post(build_scenario_grid))
        .route("/api/strategy-simulation", post(simulate_strategy))
        .route("/api/portfolios", post(create_portfolio))
        .route(
            "/api/portfolios/{id}/cash-movements",
            post(record_cash_movement),
        )
        .route(
            "/api/portfolios/{id}/option-trades",
            post(record_option_trade),
        )
        .route(
            "/api/portfolios/{id}/currency-exchanges",
            post(record_currency_exchange),
        )
        .route("/api/portfolios/{id}/balance", get(check_balance))
        .route("/api/portfolios/{id}/positions", get(list_positions))
        .route("/api/portfolios/{id}/transactions", get(list_transactions))
        .route(
            "/api/saved-strategies",
            get(list_saved_strategies).post(save_strategy),
        )
        .route(
            "/api/saved-strategies/{id}",
            axum::routing::delete(delete_strategy),
        )
        .route("/api/tracked-tickers", get(list_tracked_tickers))
        .route("/api/underlyings/resolve", get(resolve_underlying))
        .route(
            "/api/tracked-tickers/{ticker}",
            axum::routing::put(configure_tracked_ticker),
        )
        .route("/api/market/sectors", get(view_sector_performance))
        .route("/api/data-refresh/status", get(data_refresh_status))
        .route("/api/data-refresh", post(request_data_refresh))
        .layer(middleware::from_fn(canonical_error_boundary));

    let compatibility = Router::new()
        // Deprecated structural aliases retain their historical wire contracts.
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
        .route(
            "/tracked-tickers/{ticker}",
            axum::routing::put(configure_tracked_ticker),
        );

    canonical.merge(compatibility).with_state(HttpState {
        market_data: market_viewing.market_data,
        options: options_viewing.options,
        gamma_exposure: options_viewing.gamma_exposure,
        simulation,
        portfolios,
        synchronization: synchronization.synchronization,
        saved_strategies,
        tracked_tickers: tracked_tickers.management,
        underlying_resolver: tracked_tickers.resolution,
        sector_performance: market_viewing.sector_performance,
        data_refresh: synchronization.data_refresh,
    })
}

struct UnconfiguredGammaExposure;

#[async_trait::async_trait]
impl ForViewingGammaExposure for UnconfiguredGammaExposure {
    async fn gamma_exposure(
        &self,
        _request: GammaExposureRequest,
    ) -> crate::hexagon::PortResult<crate::hexagon::domain::gamma_exposure::GammaExposureAnalysis>
    {
        Err(PortError::Unavailable(
            "gamma exposure is not configured".to_string(),
        ))
    }
}

async fn view_gamma_exposure(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Query(query): Query<api_models::GammaExposureQuery>,
) -> Result<Json<api_models::GammaExposureResponse>, HttpError> {
    state
        .gamma_exposure
        .gamma_exposure(GammaExposureRequest {
            ticker,
            range_percent: query.range_percent.unwrap_or(20.0),
            points: query.points.unwrap_or(81),
            valuation_time: Utc::now(),
        })
        .await
        .map(map_gamma_exposure)
        .map(Json)
        .map_err(HttpError)
}

fn map_gamma_exposure(
    analysis: crate::hexagon::domain::gamma_exposure::GammaExposureAnalysis,
) -> api_models::GammaExposureResponse {
    use crate::hexagon::domain::gamma_exposure::{ExclusionReason as R, SnapshotOrigin as O};
    let reason = |value| match value {
        R::MissingSpot => api_models::GammaExposureExclusionReason::MissingSpot,
        R::InvalidSpot => api_models::GammaExposureExclusionReason::InvalidSpot,
        R::MissingGamma => api_models::GammaExposureExclusionReason::MissingGamma,
        R::InvalidGamma => api_models::GammaExposureExclusionReason::InvalidGamma,
        R::MissingOpenInterest => api_models::GammaExposureExclusionReason::MissingOpenInterest,
        R::InvalidOpenInterest => api_models::GammaExposureExclusionReason::InvalidOpenInterest,
        R::MissingMultiplier => api_models::GammaExposureExclusionReason::MissingMultiplier,
        R::InvalidMultiplier => api_models::GammaExposureExclusionReason::InvalidMultiplier,
        R::InvalidStrike => api_models::GammaExposureExclusionReason::InvalidStrike,
        R::ExpiredContract => api_models::GammaExposureExclusionReason::ExpiredContract,
        R::MissingImpliedVolatility => {
            api_models::GammaExposureExclusionReason::MissingImpliedVolatility
        }
        R::InvalidImpliedVolatility => {
            api_models::GammaExposureExclusionReason::InvalidImpliedVolatility
        }
        R::MissingForwardCarry => api_models::GammaExposureExclusionReason::MissingForwardCarry,
        R::NumericOverflow => api_models::GammaExposureExclusionReason::NumericOverflow,
    };
    let exposure = analysis.current_exposure;
    let current_exposure = api_models::CurrentGammaExposureResponse {
        ticker: exposure.ticker,
        spot: exposure.spot,
        currency: exposure.currency,
        as_of: exposure.as_of,
        snapshot_origin: match exposure.snapshot_origin {
            O::Intraday => api_models::GammaExposureSnapshotOrigin::Intraday,
            O::EndOfDay => api_models::GammaExposureSnapshotOrigin::EndOfDay,
        },
        calls_gex: exposure.calls_gex,
        puts_gex: exposure.puts_gex,
        net_gex: exposure.net_gex,
        by_strike: exposure
            .by_strike
            .into_iter()
            .map(|bucket| api_models::GammaExposureStrike {
                strike: bucket.key,
                calls_gex: bucket.calls_gex,
                puts_gex: bucket.puts_gex,
                net_gex: bucket.net_gex,
            })
            .collect(),
        by_expiration: exposure
            .by_expiration
            .into_iter()
            .map(|bucket| api_models::GammaExposureExpiration {
                expiration: bucket.key,
                calls_gex: bucket.calls_gex,
                puts_gex: bucket.puts_gex,
                net_gex: bucket.net_gex,
            })
            .collect(),
        methodology: exposure.methodology.to_string(),
        sign_convention: exposure.sign_convention.to_string(),
        diagnostics: api_models::GammaExposureDiagnostics {
            total_contracts: exposure.diagnostics.total_contracts,
            included_contracts: exposure.diagnostics.included_contracts,
            excluded_contracts: exposure.diagnostics.excluded_contracts,
            excluded_by_reason: exposure
                .diagnostics
                .excluded_by_reason
                .into_iter()
                .map(|(value, count)| api_models::GammaExposureExclusionCount {
                    reason: reason(value),
                    count,
                })
                .collect(),
            exclusion_samples: exposure
                .diagnostics
                .samples
                .into_iter()
                .map(|sample| api_models::GammaExposureExclusionSample {
                    occ_symbol: sample.occ_symbol,
                    reasons: sample.reasons.into_iter().map(reason).collect(),
                })
                .collect(),
            exclusion_sample_limit: exposure.diagnostics.sample_limit,
        },
    };
    let Some(modeled) = analysis.modeled_profile else {
        return api_models::GammaExposureResponse {
            current_exposure,
            modeled_profile: api_models::DataState::Unavailable,
        };
    };
    let total_contracts = modeled.included_contracts + modeled.excluded_contracts;
    let diagnostics = api_models::GammaExposureDiagnostics {
        total_contracts,
        included_contracts: modeled.included_contracts,
        excluded_contracts: modeled.excluded_contracts,
        excluded_by_reason: modeled
            .excluded_by_reason
            .into_iter()
            .map(|(value, count)| api_models::GammaExposureExclusionCount {
                reason: reason(value),
                count,
            })
            .collect(),
        exclusion_samples: modeled
            .samples
            .into_iter()
            .map(|sample| api_models::GammaExposureExclusionSample {
                occ_symbol: sample.occ_symbol,
                reasons: sample.reasons.into_iter().map(reason).collect(),
            })
            .collect(),
        exclusion_sample_limit: modeled.sample_limit,
    };
    api_models::GammaExposureResponse {
        current_exposure,
        modeled_profile: api_models::DataState::Available(
            api_models::ModeledGammaExposureProfile {
                valuation_time: modeled.valuation_time,
                range_percent: modeled.range_percent,
                points: modeled.points,
                methodology: modeled.methodology.to_string(),
                sticky_strike_assumption: modeled.sticky_strike_assumption.to_string(),
                included_contracts: modeled.included_contracts,
                excluded_contracts: modeled.excluded_contracts,
                diagnostics,
                profile: modeled
                    .profile
                    .into_iter()
                    .map(|point| api_models::ModeledGammaExposurePoint {
                        spot: point.spot,
                        call_gex: point.call_gex,
                        put_gex: point.put_gex,
                        net_gex: point.net_gex,
                    })
                    .collect(),
                zero_crossings: modeled.zero_crossings,
                nearest_zero_crossing: modeled.nearest_zero_crossing,
            },
        ),
    }
}

async fn canonical_error_boundary(request: Request<Body>, next: Next) -> Response {
    let response = next.run(request).await;
    if !response.status().is_client_error() && !response.status().is_server_error() {
        return response;
    }
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
    {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let message = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => error.to_string(),
    };
    let error = api_models::ApiError {
        error: if message.is_empty() {
            parts
                .status
                .canonical_reason()
                .unwrap_or("request failed")
                .to_string()
        } else {
            message
        },
    };
    let body = serde_json::to_vec(&error)
        .unwrap_or_else(|_| br#"{"error":"failed to serialize error response"}"#.to_vec());
    parts.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    Response::from_parts(parts, Body::from(body))
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
    use crate::hexagon::driving_ports::for_refreshing_market_data::{
        DataRefreshTrigger, StartDataRefreshResult,
    };
    match refresh_port(&state)?
        .request_data_refresh(DataRefreshTrigger::Manual, chrono::Utc::now())
        .await
        .map_err(HttpError)?
    {
        StartDataRefreshResult::Started(run) => Ok((
            StatusCode::ACCEPTED,
            Json(api_models::DataRefreshRequestResponse {
                result: api_models::DataRefreshRequestState::Started,
                run: Some(map_refresh_run(run)),
                message: "Atualização manual iniciada".to_string(),
            }),
        )),
        StartDataRefreshResult::AlreadyRunning(run) => Ok((
            StatusCode::CONFLICT,
            Json(api_models::DataRefreshRequestResponse {
                result: api_models::DataRefreshRequestState::AlreadyRunning,
                run: Some(map_refresh_run(run)),
                message: "Já existe uma atualização em curso".to_string(),
            }),
        )),
        StartDataRefreshResult::NoEligibleSession => Ok((
            StatusCode::CONFLICT,
            Json(api_models::DataRefreshRequestResponse {
                result: api_models::DataRefreshRequestState::NoEligibleSession,
                run: None,
                message: "Não existe uma sessão de mercado concluída elegível".to_string(),
            }),
        )),
    }
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

async fn view_sector_performance(
    State(state): State<HttpState>,
    Query(query): Query<api_models::SectorPerformanceQuery>,
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
    Json(request): Json<api_models::SynchronizeTrackedTickersRequest>,
) -> Result<Json<api_models::TrackedTickersSynchronizationReport>, HttpError> {
    state
        .synchronization
        .synchronize_tracked_tickers(SynchronizeTrackedTickers {
            since: request.since,
            market_close: request.market_close,
        })
        .await
        .map(canonical_models::tracked_synchronization_report)
        .map(Json)
        .map_err(HttpError)
}

async fn list_tracked_tickers(
    State(state): State<HttpState>,
    Query(query): Query<api_models::TrackedTickersQuery>,
) -> Result<Json<Vec<api_models::TrackedTicker>>, HttpError> {
    state
        .tracked_tickers
        .list_tickers(query.include_inactive)
        .await
        .map(|tickers| {
            tickers
                .into_iter()
                .map(canonical_models::tracked_ticker)
                .collect()
        })
        .map(Json)
        .map_err(HttpError)
}

async fn resolve_underlying(
    State(state): State<HttpState>,
    Query(query): Query<api_models::ResolveUnderlyingQuery>,
) -> Result<Json<api_models::UnderlyingResolution>, HttpError> {
    state
        .underlying_resolver
        .resolve_underlying(&query.ticker)
        .await
        .map(canonical_models::underlying_resolution)
        .map(Json)
        .map_err(HttpError)
}

async fn configure_tracked_ticker(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Json(body): Json<api_models::ConfigureTrackedTickerRequest>,
) -> Result<StatusCode, HttpError> {
    state
        .tracked_tickers
        .configure_ticker(
            &ticker,
            canonical_models::tracked_ticker_configuration(body),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn list_saved_strategies(
    State(state): State<HttpState>,
) -> Result<Json<Vec<api_models::SavedStrategy>>, HttpError> {
    state
        .saved_strategies
        .list_strategies()
        .await
        .map(|strategies| {
            strategies
                .into_iter()
                .map(canonical_models::saved_strategy)
                .collect()
        })
        .map(Json)
        .map_err(HttpError)
}

async fn save_strategy(
    State(state): State<HttpState>,
    Json(command): Json<api_models::SaveStrategy>,
) -> Result<Json<api_models::SavedStrategy>, HttpError> {
    state
        .saved_strategies
        .save_strategy(canonical_models::save_strategy(command))
        .await
        .map(canonical_models::saved_strategy)
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

async fn synchronize_market_history(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Json(body): Json<api_models::MarketHistorySynchronizationRequest>,
) -> Result<Json<api_models::SynchronizationReport>, HttpError> {
    state
        .synchronization
        .synchronize_market_history(&ticker, body.since)
        .await
        .map(canonical_models::synchronization_report)
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_option_chain(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
    Json(body): Json<api_models::OptionChainSynchronizationRequest>,
) -> Result<Json<api_models::SynchronizationReport>, HttpError> {
    state
        .synchronization
        .synchronize_option_chain(&ticker, body.market_close)
        .await
        .map(canonical_models::synchronization_report)
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_term_structure(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::SynchronizationReport>, HttpError> {
    state
        .synchronization
        .synchronize_term_structure(&ticker)
        .await
        .map(canonical_models::synchronization_report)
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_volatility_index(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::SynchronizationReport>, HttpError> {
    state
        .synchronization
        .synchronize_volatility_index(&ticker)
        .await
        .map(canonical_models::synchronization_report)
        .map(Json)
        .map_err(HttpError)
}

async fn synchronize_yield_curves(
    State(state): State<HttpState>,
    Path(year): Path<i32>,
) -> Result<Json<api_models::SynchronizationReport>, HttpError> {
    state
        .synchronization
        .synchronize_yield_curves(year)
        .await
        .map(canonical_models::synchronization_report)
        .map(Json)
        .map_err(HttpError)
}

async fn create_portfolio(
    State(state): State<HttpState>,
    Json(body): Json<api_models::CreatePortfolioRequest>,
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
    Json(movement): Json<api_models::CashMovement>,
) -> Result<StatusCode, HttpError> {
    state
        .portfolios
        .record_cash_movement(
            &id,
            canonical_models::cash_movement(movement).map_err(HttpError)?,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn record_option_trade(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    Json(trade): Json<api_models::Trade>,
) -> Result<StatusCode, HttpError> {
    state
        .portfolios
        .record_option_trade(&id, canonical_models::trade(trade).map_err(HttpError)?)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn record_currency_exchange(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    Json(exchange): Json<api_models::CurrencyExchange>,
) -> Result<StatusCode, HttpError> {
    state
        .portfolios
        .record_currency_exchange(
            &id,
            canonical_models::currency_exchange(exchange).map_err(HttpError)?,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(HttpError)
}

async fn check_balance(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<api_models::PortfolioBalance>, HttpError> {
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
) -> Result<Json<Vec<api_models::Position>>, HttpError> {
    state
        .portfolios
        .list_positions(&id)
        .await
        .map(|positions| {
            positions
                .into_iter()
                .map(canonical_models::position)
                .collect()
        })
        .map(Json)
        .map_err(HttpError)
}

async fn list_transactions(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<api_models::PortfolioEvent>>, HttpError> {
    state
        .portfolios
        .list_transactions(&id)
        .await
        .map(|events| {
            events
                .into_iter()
                .map(canonical_models::portfolio_event)
                .collect()
        })
        .map(Json)
        .map_err(HttpError)
}

async fn build_scenario_grid(
    State(state): State<HttpState>,
    Json(request): Json<api_models::ScenarioGridRequest>,
) -> Result<Json<api_models::ScenarioGrid>, HttpError> {
    state
        .simulation
        .build_scenario_grid(canonical_models::scenario_grid_request(request))
        .await
        .map(canonical_models::scenario_grid)
        .map(Json)
        .map_err(HttpError)
}

async fn simulate_strategy(
    State(state): State<HttpState>,
    Json(request): Json<api_models::StrategySimulationRequest>,
) -> Result<Json<api_models::StrategySimulationResult>, HttpError> {
    state
        .simulation
        .simulate_strategy(canonical_models::simulation_request(request))
        .await
        .map(canonical_models::simulation_result)
        .map(Json)
        .map_err(HttpError)
}

async fn market_history(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::MarketHistory>, HttpError> {
    state
        .market_data
        .market_history(&ticker)
        .await
        .map(canonical_models::market_history)
        .map(Json)
        .map_err(HttpError)
}

async fn live_price(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::LivePrice>, HttpError> {
    state
        .market_data
        .live_price(&ticker)
        .await
        .map(canonical_models::live_price)
        .map(Json)
        .map_err(HttpError)
}

async fn option_chain(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::OptionSnapshot>, HttpError> {
    state
        .options
        .option_chain(&ticker)
        .await
        .map(canonical_models::option_snapshot)
        .map(Json)
        .map_err(HttpError)
}

async fn term_structure(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::TermStructure>, HttpError> {
    state
        .options
        .term_structure(&ticker)
        .await
        .map(canonical_models::term_structure)
        .map(Json)
        .map_err(HttpError)
}

async fn volatility_surface(
    State(state): State<HttpState>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::VolatilitySurface>, HttpError> {
    state
        .options
        .volatility_surface(&ticker)
        .await
        .map(canonical_models::volatility_surface)
        .map(Json)
        .map_err(HttpError)
}

async fn volatility_skew(
    State(state): State<HttpState>,
    Path((ticker, expiration)): Path<(String, String)>,
) -> Result<Json<api_models::VolatilitySkew>, HttpError> {
    let expiration = NaiveDate::parse_from_str(&expiration, "%Y-%m-%d")
        .map_err(|_| HttpError(PortError::InvalidRequest("invalid expiration".into())))?;
    state
        .options
        .volatility_skew(&ticker, expiration)
        .await
        .map(canonical_models::volatility_skew)
        .map(Json)
        .map_err(HttpError)
}

async fn greeks(
    State(state): State<HttpState>,
    Path((ticker, occ_symbol)): Path<(String, String)>,
) -> Result<Json<api_models::Greeks>, HttpError> {
    state
        .options
        .greeks(GreeksRequest { ticker, occ_symbol })
        .await
        .map(canonical_models::greeks)
        .map(Json)
        .map_err(HttpError)
}

#[derive(Debug)]
struct HttpError(PortError);

impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            PortError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            PortError::NotFound(_) => StatusCode::NOT_FOUND,
            PortError::Conflict(_) => StatusCode::CONFLICT,
            PortError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = api_models::ApiError {
            error: self.0.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
