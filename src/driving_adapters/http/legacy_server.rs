use crate::hexagon::domain::tracked_ticker::TrackedTicker;
use crate::hexagon::{
    PortError,
    domain::portfolio::{
        CashMovement, CashMovementKind, Currency, CurrencyExchange, ExchangeRate, Instrument,
        Money, Portfolio, Trade, TradeSide, decimal,
    },
    domain::saved_strategy::{SavedStrategy, SavedStrategyLeg, StrategySide},
    driving_ports::for_analyzing_options::ForAnalyzingOptions,
    driving_ports::for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
    driving_ports::for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
    driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers,
    driving_ports::for_preparing_intraday_simulations::ForPreparingIntradaySimulations,
    driving_ports::for_simulating_strategies::{ForSimulatingStrategies, SimulateScenario},
    driving_ports::for_streaming_market_prices::ForStreamingMarketPrices,
    driving_ports::for_viewing_interest_rates::ForViewingInterestRates,
    driving_ports::for_viewing_intraday_options::ForViewingIntradayOptions,
    driving_ports::for_viewing_market_data::ForViewingMarketData,
    driving_ports::for_viewing_portfolio_positions::ForViewingPortfolioPositions,
    driving_ports::for_viewing_volatility::ForViewingVolatility,
};
use api_models::{AssetKind, AssetLivePrice, AssetSummary};
use axum::{
    Extension, Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message as WebSocketMessage, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::Response,
    routing::get,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct LegacyHttpPorts {
    pub market_data: Arc<dyn ForViewingMarketData>,
    pub market_stream: Arc<dyn ForStreamingMarketPrices>,
    pub market_volatility: Arc<dyn ForViewingVolatility>,
    pub interest_rates: Arc<dyn ForViewingInterestRates>,
    pub portfolios: Arc<dyn ForManagingPortfolios>,
    pub portfolio_valuation: Arc<dyn ForViewingPortfolioPositions>,
    pub options: Arc<dyn ForAnalyzingOptions>,
    pub simulation: Arc<dyn ForSimulatingStrategies>,
    pub intraday_simulation: Arc<dyn ForPreparingIntradaySimulations>,
    pub intraday_options: Arc<dyn ForViewingIntradayOptions>,
    pub saved_strategies: Arc<dyn ForManagingSavedStrategies>,
    pub tracked_tickers: Arc<dyn ForManagingTrackedTickers>,
}

#[derive(serde::Deserialize)]
struct SimulationQuery {
    ticker: Option<String>,
}

#[derive(serde::Deserialize)]
struct LivePriceQuery {
    ticker: Option<String>,
}

#[derive(serde::Deserialize)]
struct LivePriceSubscription {
    ticker: String,
}

pub fn router(
    ports: LegacyHttpPorts,
    market_session: tokio::sync::watch::Receiver<bool>,
) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/assets", get(list_assets))
        .route("/api/assets/live", get(asset_live_prices))
        .route("/api/market/benchmark", get(market_benchmark))
        .route("/api/market/volatility", get(market_volatility))
        .route("/api/market/spx-history", get(market_spx_history))
        .route("/api/market/vix-history", get(market_vix_history))
        .route("/api/market/rates", get(market_rates))
        .route(
            "/api/portfolio",
            get(portfolio_overview).post(create_portfolio_cash_movement),
        )
        .route("/api/portfolio/summary", get(portfolio_summary))
        .route("/api/portfolio/cash", get(portfolio_cash))
        .route("/api/portfolio/positions", get(portfolio_positions))
        .route("/api/portfolio/movements", get(portfolio_movements))
        .route(
            "/api/simulation",
            get(simulation_overview).post(simulate_scenarios),
        )
        .route(
            "/api/simulation/intraday",
            axum::routing::post(simulate_intraday_scenarios),
        )
        .route("/api/strategies", get(saved_strategies).post(save_strategy))
        .route(
            "/api/strategies/{id}",
            axum::routing::delete(delete_strategy),
        )
        .route("/api/simulation/contracts", get(simulation_contracts))
        .route(
            "/api/portfolio/option-trades",
            axum::routing::post(create_portfolio_option_trade),
        )
        .route(
            "/api/portfolio/currency-exchanges",
            axum::routing::post(create_portfolio_currency_exchange),
        )
        .route("/api/assets/{ticker}/price", get(asset_price))
        .route(
            "/api/assets/{ticker}/price-history",
            get(asset_price_history),
        )
        .route(
            "/api/assets/{ticker}/historical-volatility",
            get(asset_historical_volatility),
        )
        .route(
            "/api/assets/{ticker}/implied-volatility",
            get(asset_implied_volatility),
        )
        .route(
            "/api/assets/{ticker}/options/snapshot",
            get(options_snapshot),
        )
        .route(
            "/api/assets/{ticker}/options/term-structure",
            get(options_term_structure),
        )
        .route(
            "/api/assets/{ticker}/options/volatility-surface",
            get(options_volatility_surface),
        )
        .route(
            "/api/assets/{ticker}/options/intraday",
            get(options_intraday),
        )
        .with_state(ports)
        .layer(Extension(market_session))
}

async fn health() -> &'static str {
    "ok"
}

async fn list_assets(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<Vec<AssetSummary>>, StatusCode> {
    let tracked = state
        .tracked_tickers
        .list_tickers(false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tracked.iter().map(asset_summary).collect()))
}

async fn market_benchmark(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::MarketBenchmarkResponse>, StatusCode> {
    let application = state.market_data;
    let vix = application
        .index_history("VIX")
        .await
        .map_err(port_status)?;
    let history = application
        .market_history("SPY")
        .await
        .map_err(port_status)?;
    let as_of = latest_index_date(&vix)?;
    Ok(Json(
        crate::driving_adapters::http::legacy_market_views::benchmark(&history, as_of),
    ))
}

async fn market_volatility(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::MarketVolatilityResponse>, StatusCode> {
    state
        .market_volatility
        .volatility_overview()
        .await
        .map(crate::driving_adapters::http::legacy_market_views::volatility)
        .map(Json)
        .map_err(port_status)
}

async fn market_spx_history(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::MarketSpxHistoryResponse>, StatusCode> {
    let application = state.market_data;
    let vix = application
        .index_history("VIX")
        .await
        .map_err(port_status)?;
    let history = application
        .market_history("SPX")
        .await
        .map_err(port_status)?;
    let as_of = latest_index_date(&vix)?;
    Ok(Json(
        crate::driving_adapters::http::legacy_market_views::spx_history(&history, as_of),
    ))
}

async fn market_vix_history(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::MarketVixHistoryResponse>, StatusCode> {
    let history = state
        .market_data
        .index_history("VIX")
        .await
        .map_err(port_status)?;
    let as_of = latest_index_date(&history)?;
    Ok(Json(
        crate::driving_adapters::http::legacy_market_views::vix_history(&history, as_of),
    ))
}

fn latest_index_date(
    history: &crate::hexagon::domain::index_history::IndexHistory,
) -> Result<chrono::NaiveDate, StatusCode> {
    history
        .daily_prices
        .last()
        .map(|price| price.date)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn port_status(error: PortError) -> StatusCode {
    match error {
        PortError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
        PortError::NotFound(_) => StatusCode::NOT_FOUND,
        PortError::Conflict(_) => StatusCode::CONFLICT,
        PortError::Unavailable(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn market_rates(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::MarketRatesResponse>, StatusCode> {
    let vix = state
        .market_data
        .index_history("VIX")
        .await
        .map_err(port_status)?;
    let as_of = latest_index_date(&vix)?;
    let curve = state
        .interest_rates
        .yield_curve(as_of)
        .await
        .map_err(port_status)?;
    crate::driving_adapters::http::legacy_market_views::rates(as_of, curve.as_ref())
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn asset_live_prices(
    State(state): State<LegacyHttpPorts>,
    websocket: WebSocketUpgrade,
    Query(query): Query<LivePriceQuery>,
    Extension(market_session): Extension<tokio::sync::watch::Receiver<bool>>,
) -> Response {
    let ticker = query.ticker.unwrap_or_else(|| "SPX".to_string());
    websocket.on_upgrade(move |socket| {
        handle_asset_live_prices(
            socket,
            ticker,
            market_session,
            state.market_data,
            state.market_stream,
        )
    })
}

async fn handle_asset_live_prices(
    mut socket: WebSocket,
    ticker: String,
    mut market_session: tokio::sync::watch::Receiver<bool>,
    market_data: Arc<dyn ForViewingMarketData>,
    market_stream: Arc<dyn ForStreamingMarketPrices>,
) {
    let (subscription_updates, subscription) = tokio::sync::watch::channel(ticker);
    let (prices, mut received_prices) = tokio::sync::mpsc::channel(32);
    let seed_prices = prices.clone();
    let seed_ticker = subscription.borrow().clone();
    let seed_application = market_data.clone();
    tokio::spawn(async move {
        if let Ok(price) = seed_application.live_price(&seed_ticker).await {
            let _ = seed_prices.send(price).await;
        }
    });
    let stream_application = market_stream;
    let mut market_stream = tokio::spawn(async move {
        stream_application
            .stream_market_prices(subscription, prices)
            .await
    });
    let mut regular_session = *market_session.borrow();
    let mut last_price = None;

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(WebSocketMessage::Text(text))) => {
                        let Ok(subscription) =
                            serde_json::from_str::<LivePriceSubscription>(&text)
                        else {
                            continue;
                        };
                        if subscription_updates.send(subscription.ticker).is_err() {
                            break;
                        }
                    }
                    Some(Ok(WebSocketMessage::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
            price = received_prices.recv() => {
                let Some(price) = price else {
                    break;
                };
                last_price = Some(price.clone());
                let response = AssetLivePrice {
                    ticker: price.ticker,
                    price: price.price,
                    market_time: price.market_time,
                    currency: price.currency,
                    exchange: price.exchange,
                    market_hours: i32::from(price.regular_session),
                    change: price.change,
                    change_percent: price.change_percent,
                    day_volume: price.day_volume,
                };
                let Ok(payload) = serde_json::to_string(&response) else {
                    continue;
                };
                if socket
                    .send(WebSocketMessage::Text(payload.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            changed = market_session.changed() => {
                if changed.is_err() {
                    break;
                }
                let current_session = *market_session.borrow_and_update();
                if regular_session && !current_session {
                    let price = match last_price.clone() {
                        Some(mut price) => {
                            price.regular_session = false;
                            Some(price)
                        }
                        None => {
                            let ticker = subscription_updates.borrow().clone();
                            market_data
                                .live_price(&ticker)
                                .await
                                .ok()
                        }
                    };
                    if let Some(price) = price {
                        let response = AssetLivePrice {
                            ticker: price.ticker,
                            price: price.price,
                            market_time: price.market_time,
                            currency: price.currency,
                            exchange: price.exchange,
                            market_hours: 0,
                            change: price.change,
                            change_percent: price.change_percent,
                            day_volume: price.day_volume,
                        };
                        let Ok(payload) = serde_json::to_string(&response) else {
                            continue;
                        };
                        if socket
                            .send(WebSocketMessage::Text(payload.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                regular_session = current_session;
            }
            result = &mut market_stream => {
                if let Ok(Err(error)) = result {
                    eprintln!("Market live stream terminou: {error}");
                }
                return;
            }
        }
    }

    market_stream.abort();
    let _ = market_stream.await;
}

async fn portfolio_overview(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::PortfolioOverview>, StatusCode> {
    let application = state.portfolios;
    let portfolio = main_portfolio(application.as_ref()).await?;
    crate::driving_adapters::http::legacy_portfolio_views::overview(portfolio)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn portfolio_summary(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::PortfolioSummaryResponse>, StatusCode> {
    let application = state.portfolios;
    let portfolio = main_portfolio(application.as_ref()).await?;
    crate::driving_adapters::http::legacy_portfolio_views::summary(&portfolio)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn portfolio_cash(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::PortfolioCashResponse>, StatusCode> {
    let application = state.portfolios;
    let portfolio = main_portfolio(application.as_ref()).await?;
    Ok(Json(
        crate::driving_adapters::http::legacy_portfolio_views::cash(&portfolio),
    ))
}

async fn portfolio_positions(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::PortfolioPositionsResponse>, StatusCode> {
    main_portfolio(state.portfolios.as_ref()).await?;
    state
        .portfolio_valuation
        .valued_positions("main")
        .await
        .map(|positions| {
            Json(api_models::PortfolioPositionsResponse {
                positions: positions
                    .into_iter()
                    .map(|position| {
                        let (market_price, market_currency, market_source, market_time) = position
                            .market_price
                            .map(|price| {
                                (
                                    Some(price.price),
                                    Some(price.currency),
                                    Some(price.source),
                                    Some(price.observed_at),
                                )
                            })
                            .unwrap_or((None, None, None, None));
                        api_models::PortfolioPositionOverview {
                            instrument: match position.instrument {
                                Instrument::Equity { ticker } => ticker,
                                Instrument::Option { occ_symbol } => occ_symbol,
                            },
                            quantity: position.quantity.to_string(),
                            market_price,
                            market_value: position.market_value,
                            market_currency,
                            market_source,
                            market_time,
                        }
                    })
                    .collect(),
            })
        })
        .map_err(port_status)
}

async fn portfolio_movements(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<api_models::PortfolioMovementsResponse>, StatusCode> {
    let application = state.portfolios;
    let portfolio = main_portfolio(application.as_ref()).await?;
    crate::driving_adapters::http::legacy_portfolio_views::movements(&portfolio)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn main_portfolio(application: &dyn ForManagingPortfolios) -> Result<Portfolio, StatusCode> {
    match application.portfolio("main").await {
        Ok(portfolio) => Ok(portfolio),
        Err(PortError::NotFound(_)) => {
            application
                .create_portfolio(CreatePortfolio {
                    id: "main".to_string(),
                    name: "Portfolio principal".to_string(),
                    base_currency: Currency::eur(),
                })
                .await
                .map_err(port_status)?;
            application.portfolio("main").await.map_err(port_status)
        }
        Err(error) => Err(port_status(error)),
    }
}

async fn simulation_overview(
    State(state): State<LegacyHttpPorts>,
    Query(query): Query<SimulationQuery>,
) -> Result<Json<api_models::SimulationOverview>, StatusCode> {
    let ticker = query.ticker.as_deref().unwrap_or("SPX");
    let (snapshot, spot, curve) = simulation_domain_inputs(&state, ticker).await?;
    state
        .simulation
        .simulate_scenario(SimulateScenario {
            ticker: ticker.trim().to_ascii_uppercase(),
            snapshot,
            spot,
            yield_curve: curve,
            valuation_dates: None,
            strategy_kind: crate::hexagon::domain::simulation::SimulationStrategyKind::Straddle,
            volatility_shifts: vec![-0.10, 0.0, 0.10],
            legs: Vec::new(),
        })
        .await
        .map(crate::driving_adapters::http::legacy_simulation_views::scenario)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn simulation_contracts(
    State(state): State<LegacyHttpPorts>,
    Query(query): Query<SimulationQuery>,
) -> Result<Json<api_models::SimulationCatalogOverview>, StatusCode> {
    let ticker = query.ticker.as_deref().unwrap_or("SPX");
    let snapshot = state
        .options
        .option_chain(ticker)
        .await
        .map_err(port_status)?;
    let history = state
        .market_data
        .market_history(ticker)
        .await
        .map_err(port_status)?;
    let valuation_date = snapshot.timestamp_utc.date_naive();
    let spot = history
        .daily_quotes
        .iter()
        .rev()
        .find(|quote| quote.timestamp.date_naive() <= valuation_date)
        .and_then(|quote| quote.close)
        .filter(|price| price.is_finite() && *price > 0.0)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(
        crate::driving_adapters::http::legacy_simulation_views::catalog(
            &ticker.trim().to_ascii_uppercase(),
            &snapshot,
            spot,
        ),
    ))
}

async fn simulate_scenarios(
    State(state): State<LegacyHttpPorts>,
    Json(request): Json<api_models::SimulationScenarioRequest>,
) -> Result<Json<api_models::SimulationOverview>, StatusCode> {
    let (snapshot, spot, curve) = simulation_domain_inputs(&state, &request.ticker).await?;
    let strategy_kind = domain_strategy_kind(request.strategy_kind);
    let legs = domain_simulation_legs(&request.legs);
    state
        .simulation
        .simulate_scenario(SimulateScenario {
            ticker: request.ticker.trim().to_ascii_uppercase(),
            snapshot,
            spot,
            yield_curve: curve,
            valuation_dates: Some(request.valuation_dates),
            strategy_kind,
            volatility_shifts: request.volatility_shifts,
            legs,
        })
        .await
        .map(crate::driving_adapters::http::legacy_simulation_views::scenario)
        .map(Json)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn simulation_domain_inputs(
    state: &LegacyHttpPorts,
    ticker: &str,
) -> Result<
    (
        crate::hexagon::domain::options::Snapshot,
        f64,
        crate::hexagon::domain::treasury::YieldCurve,
    ),
    StatusCode,
> {
    let snapshot = state
        .options
        .option_chain(ticker)
        .await
        .map_err(port_status)?;
    let valuation_date = snapshot.timestamp_utc.date_naive();
    let history = state
        .market_data
        .market_history(ticker)
        .await
        .map_err(port_status)?;
    let spot = history
        .daily_quotes
        .iter()
        .rev()
        .find(|quote| quote.timestamp.date_naive() <= valuation_date)
        .and_then(|quote| quote.close)
        .filter(|price| price.is_finite() && *price > 0.0)
        .ok_or(StatusCode::NOT_FOUND)?;
    let curve = state
        .interest_rates
        .yield_curve(valuation_date)
        .await
        .map_err(port_status)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok((snapshot, spot, curve))
}

async fn simulate_intraday_scenarios(
    State(state): State<LegacyHttpPorts>,
    Json(request): Json<api_models::SimulationScenarioRequest>,
) -> Result<Json<api_models::SimulationOverview>, StatusCode> {
    let market = state
        .intraday_simulation
        .intraday_market(&request.ticker)
        .await
        .map_err(port_status)?;
    let valuation_date = market.snapshot.timestamp_utc.date_naive();
    let curve = state
        .interest_rates
        .yield_curve(valuation_date)
        .await
        .map_err(port_status)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let strategy_kind = domain_strategy_kind(request.strategy_kind);
    let legs = domain_simulation_legs(&request.legs);
    state
        .simulation
        .simulate_scenario(SimulateScenario {
            ticker: request.ticker.trim().to_ascii_uppercase(),
            snapshot: market.snapshot,
            spot: market.spot,
            yield_curve: curve,
            valuation_dates: Some(request.valuation_dates),
            strategy_kind,
            volatility_shifts: request.volatility_shifts,
            legs,
        })
        .await
        .map(crate::driving_adapters::http::legacy_simulation_views::scenario)
        .map(Json)
        .map_err(|error| {
            eprintln!("Falha ao recalcular a simulação intradiária: {error}");
            StatusCode::BAD_REQUEST
        })
}

fn domain_strategy_kind(
    kind: api_models::SimulationStrategyKind,
) -> crate::hexagon::domain::simulation::SimulationStrategyKind {
    match kind {
        api_models::SimulationStrategyKind::Straddle => {
            crate::hexagon::domain::simulation::SimulationStrategyKind::Straddle
        }
        api_models::SimulationStrategyKind::BullCallSpread => {
            crate::hexagon::domain::simulation::SimulationStrategyKind::BullCallSpread
        }
        api_models::SimulationStrategyKind::Custom => {
            crate::hexagon::domain::simulation::SimulationStrategyKind::Custom
        }
    }
}

fn domain_simulation_legs(
    legs: &[api_models::SimulationLegRequest],
) -> Vec<crate::hexagon::domain::simulation::SimulationLegSelection> {
    legs.iter()
        .map(
            |leg| crate::hexagon::domain::simulation::SimulationLegSelection {
                occ_symbol: leg.occ_symbol.clone(),
                side: match leg.side {
                    api_models::SimulationTradeSide::Buy => {
                        crate::hexagon::domain::simulation::SimulationTradeSide::Buy
                    }
                    api_models::SimulationTradeSide::Sell => {
                        crate::hexagon::domain::simulation::SimulationTradeSide::Sell
                    }
                },
                quantity: leg.quantity,
                entry_price: leg.entry_price,
            },
        )
        .collect()
}

async fn saved_strategies(
    State(state): State<LegacyHttpPorts>,
) -> Result<Json<Vec<api_models::SavedStrategyOverview>>, StatusCode> {
    state
        .saved_strategies
        .list_strategies()
        .await
        .map(|strategies| {
            Json(
                strategies
                    .into_iter()
                    .map(saved_strategy_overview)
                    .collect(),
            )
        })
        .map_err(|error| {
            eprintln!("Falha ao carregar estratégias: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn save_strategy(
    State(state): State<LegacyHttpPorts>,
    Json(request): Json<api_models::SaveStrategyRequest>,
) -> Result<(StatusCode, Json<api_models::SavedStrategyOverview>), StatusCode> {
    let command = SaveStrategy {
        name: request.name,
        ticker: request.ticker,
        legs: request
            .legs
            .into_iter()
            .map(|leg| SavedStrategyLeg {
                occ_symbol: leg.occ_symbol,
                side: match leg.side {
                    api_models::SimulationTradeSide::Buy => StrategySide::Buy,
                    api_models::SimulationTradeSide::Sell => StrategySide::Sell,
                },
                quantity: leg.quantity,
                entry_price: leg.entry_price,
            })
            .collect(),
    };
    state
        .saved_strategies
        .save_strategy(command)
        .await
        .map(|strategy| (StatusCode::CREATED, Json(saved_strategy_overview(strategy))))
        .map_err(|error| {
            eprintln!("Falha ao guardar estratégia: {error}");
            StatusCode::BAD_REQUEST
        })
}

async fn delete_strategy(
    State(state): State<LegacyHttpPorts>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    state
        .saved_strategies
        .delete_strategy(id)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|error| {
            eprintln!("Falha ao eliminar estratégia: {error}");
            StatusCode::NOT_FOUND
        })
}

fn saved_strategy_overview(strategy: SavedStrategy) -> api_models::SavedStrategyOverview {
    api_models::SavedStrategyOverview {
        id: strategy.id,
        name: strategy.name,
        ticker: strategy.ticker,
        legs: strategy
            .legs
            .into_iter()
            .map(|leg| api_models::SimulationLegRequest {
                occ_symbol: leg.occ_symbol,
                side: match leg.side {
                    StrategySide::Buy => api_models::SimulationTradeSide::Buy,
                    StrategySide::Sell => api_models::SimulationTradeSide::Sell,
                },
                quantity: leg.quantity,
                entry_price: leg.entry_price,
            })
            .collect(),
        updated_at: strategy.updated_at,
    }
}

async fn create_portfolio_cash_movement(
    State(state): State<LegacyHttpPorts>,
    Json(request): Json<api_models::CreatePortfolioCashMovement>,
) -> Result<(StatusCode, Json<api_models::PortfolioOverview>), StatusCode> {
    let application = state.portfolios;
    main_portfolio(application.as_ref()).await?;
    let movement = CashMovement::new(
        request.id,
        request.occurred_at,
        match request.kind {
            api_models::PortfolioCashMovementKind::Deposit => CashMovementKind::Deposit,
            api_models::PortfolioCashMovementKind::Withdrawal => CashMovementKind::Withdrawal,
        },
        Money::new(
            decimal(&request.amount).map_err(|_| StatusCode::BAD_REQUEST)?,
            Currency::new(&request.currency).map_err(|_| StatusCode::BAD_REQUEST)?,
        ),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    application
        .record_cash_movement("main", movement)
        .await
        .map_err(port_status)?;
    let overview = crate::driving_adapters::http::legacy_portfolio_views::overview(
        application.portfolio("main").await.map_err(port_status)?,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(overview)))
}

async fn create_portfolio_option_trade(
    State(state): State<LegacyHttpPorts>,
    Json(request): Json<api_models::CreatePortfolioOptionTrade>,
) -> Result<(StatusCode, Json<api_models::PortfolioOverview>), StatusCode> {
    let application = state.portfolios;
    main_portfolio(application.as_ref()).await?;
    let currency = Currency::new(&request.currency).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut trade = Trade::new(
        request.id,
        Instrument::Option {
            occ_symbol: request.occ_symbol,
        },
        match request.side {
            api_models::PortfolioTradeSide::Buy => TradeSide::Buy,
            api_models::PortfolioTradeSide::Sell => TradeSide::Sell,
        },
        request.occurred_at,
        decimal(&request.quantity).map_err(|_| StatusCode::BAD_REQUEST)?,
        Money::new(
            decimal(&request.premium).map_err(|_| StatusCode::BAD_REQUEST)?,
            currency.clone(),
        ),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    if currency != Currency::eur() {
        let rate = ExchangeRate::new(
            currency,
            Currency::eur(),
            decimal(&request.tax_rate_to_eur).map_err(|_| StatusCode::BAD_REQUEST)?,
            request.tax_rate_date,
            request.tax_rate_source,
        )
        .map_err(|_| StatusCode::BAD_REQUEST)?;
        trade.settlement_rate_to_eur = Some(rate.clone());
        trade.tax_rate_to_eur = Some(rate);
    }
    application
        .record_option_trade("main", trade)
        .await
        .map_err(port_status)?;
    let overview = crate::driving_adapters::http::legacy_portfolio_views::overview(
        application.portfolio("main").await.map_err(port_status)?,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(overview)))
}

async fn create_portfolio_currency_exchange(
    State(state): State<LegacyHttpPorts>,
    Json(request): Json<api_models::CreatePortfolioCurrencyExchange>,
) -> Result<(StatusCode, Json<api_models::PortfolioOverview>), StatusCode> {
    let application = state.portfolios;
    main_portfolio(application.as_ref()).await?;
    let sold_currency =
        Currency::new(&request.sold_currency).map_err(|_| StatusCode::BAD_REQUEST)?;
    let bought_currency =
        Currency::new(&request.bought_currency).map_err(|_| StatusCode::BAD_REQUEST)?;
    let exchange = CurrencyExchange::new(
        request.id,
        request.occurred_at,
        Money::new(
            decimal(&request.sold_amount).map_err(|_| StatusCode::BAD_REQUEST)?,
            sold_currency.clone(),
        ),
        Money::new(
            decimal(&request.bought_amount).map_err(|_| StatusCode::BAD_REQUEST)?,
            bought_currency.clone(),
        ),
        ExchangeRate::new(
            bought_currency,
            sold_currency,
            decimal(&request.rate).map_err(|_| StatusCode::BAD_REQUEST)?,
            request.rate_date,
            request.rate_source,
        )
        .map_err(|_| StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    application
        .record_currency_exchange("main", exchange)
        .await
        .map_err(port_status)?;
    let overview = crate::driving_adapters::http::legacy_portfolio_views::overview(
        application.portfolio("main").await.map_err(port_status)?,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(overview)))
}

async fn asset_price(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::AssetPriceResponse>, StatusCode> {
    let normalized = ticker.trim().to_ascii_uppercase();
    state
        .market_data
        .market_history(&normalized)
        .await
        .map(|history| Json(legacy_asset_price(normalized, &history)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn asset_price_history(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::AssetPriceHistoryResponse>, StatusCode> {
    let normalized = ticker.trim().to_ascii_uppercase();
    state
        .market_data
        .market_history(&normalized)
        .await
        .map(|history| Json(legacy_asset_price_history(normalized, &history)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn legacy_asset_price(
    ticker: String,
    history: &crate::hexagon::domain::market_history::MarketHistory,
) -> api_models::AssetPriceResponse {
    let latest = history.daily_quotes.last();
    let as_of = latest.map(|quote| quote.timestamp.date_naive());
    let price = latest
        .and_then(|quote| {
            let close = quote.close?;
            let previous_close = history
                .daily_quotes
                .iter()
                .rev()
                .skip(1)
                .find_map(|previous| previous.close);
            Some(api_models::AssetPriceOverview {
                metadata: option_metadata(quote.timestamp.date_naive(), "market data"),
                open: quote.open,
                high: quote.high,
                low: quote.low,
                close,
                adjusted_close: quote.adjusted_close,
                volume: quote.volume,
                daily_change_pct: previous_close
                    .filter(|previous| *previous != 0.0)
                    .map(|previous| (close / previous - 1.0) * 100.0),
            })
        })
        .map(api_models::DataState::Available)
        .unwrap_or(api_models::DataState::Unavailable);
    api_models::AssetPriceResponse {
        ticker,
        as_of,
        price,
    }
}

fn legacy_asset_price_history(
    ticker: String,
    history: &crate::hexagon::domain::market_history::MarketHistory,
) -> api_models::AssetPriceHistoryResponse {
    const MAX_SESSIONS: usize = 1_260;
    let as_of = history
        .daily_quotes
        .last()
        .map(|quote| quote.timestamp.date_naive());
    let mut points = history
        .daily_quotes
        .iter()
        .rev()
        .filter_map(|quote| {
            Some(api_models::PriceHistoryPoint {
                date: quote.timestamp.date_naive(),
                open: quote.open?,
                high: quote.high?,
                low: quote.low?,
                close: quote.close?,
            })
        })
        .take(MAX_SESSIONS)
        .collect::<Vec<_>>();
    let price_history = if let Some(latest_date) = points.first().map(|point| point.date) {
        points.reverse();
        api_models::DataState::Available(api_models::PriceHistoryOverview {
            metadata: option_metadata(latest_date, "market data"),
            points,
        })
    } else {
        api_models::DataState::Unavailable
    };
    api_models::AssetPriceHistoryResponse {
        ticker,
        as_of,
        price_history,
    }
}

async fn asset_historical_volatility(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::AssetHistoricalVolatilityResponse>, StatusCode> {
    state
        .market_volatility
        .historical_volatility(&ticker)
        .await
        .map(crate::driving_adapters::http::legacy_asset_views::historical_volatility)
        .map(Json)
        .map_err(port_status)
}

async fn asset_implied_volatility(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::AssetImpliedVolatilityResponse>, StatusCode> {
    state
        .market_volatility
        .implied_volatility(&ticker)
        .await
        .map(crate::driving_adapters::http::legacy_asset_views::implied_volatility)
        .map(Json)
        .map_err(port_status)
}

async fn options_snapshot(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::OptionsSnapshotResponse>, StatusCode> {
    let normalized = ticker.trim().to_ascii_uppercase();
    match state.options.option_chain(&normalized).await {
        Ok(snapshot) => {
            let expirations = snapshot
                .contratos
                .iter()
                .map(|contract| contract.expiration)
                .collect::<std::collections::HashSet<_>>()
                .len();
            let calls = snapshot
                .contratos
                .iter()
                .filter(|contract| {
                    contract.option_type == crate::hexagon::domain::options::OptionType::Call
                })
                .count();
            let minimum_strike = snapshot
                .contratos
                .iter()
                .map(|contract| contract.strike)
                .reduce(f64::min);
            let maximum_strike = snapshot
                .contratos
                .iter()
                .map(|contract| contract.strike)
                .reduce(f64::max);
            Ok(Json(api_models::OptionsSnapshotResponse {
                ticker: normalized,
                snapshot_time: Some(snapshot.timestamp_utc),
                snapshot: api_models::DataState::Available(api_models::OptionSnapshotOverview {
                    metadata: option_metadata(snapshot.timestamp_utc.date_naive(), "market data"),
                    expirations,
                    contracts: snapshot.contratos.len(),
                    calls,
                    puts: snapshot.contratos.len() - calls,
                    minimum_strike,
                    maximum_strike,
                }),
            }))
        }
        Err(PortError::NotFound(_)) => Ok(Json(api_models::OptionsSnapshotResponse {
            ticker: normalized,
            snapshot_time: None,
            snapshot: api_models::DataState::Unavailable,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn options_term_structure(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::OptionsTermStructureResponse>, StatusCode> {
    let normalized = ticker.trim().to_ascii_uppercase();
    match state.options.term_structure(&normalized).await {
        Ok(term) => Ok(Json(api_models::OptionsTermStructureResponse {
            ticker: normalized,
            snapshot_time: Some(term.snapshot_timestamp),
            term_structure: api_models::DataState::Available(
                api_models::OptionTermStructureOverview {
                    metadata: option_metadata(term.snapshot_timestamp.date_naive(), "market data"),
                    treasury_date: term.treasury_date,
                    points: term
                        .points
                        .into_iter()
                        .map(|point| api_models::OptionTermStructurePoint {
                            days: point.days,
                            volatility_percent: point.volatility,
                        })
                        .collect(),
                },
            ),
        })),
        Err(PortError::NotFound(_)) => Ok(Json(api_models::OptionsTermStructureResponse {
            ticker: normalized,
            snapshot_time: None,
            term_structure: api_models::DataState::Unavailable,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn option_metadata(session_date: chrono::NaiveDate, source: &str) -> api_models::ViewMetadata {
    api_models::ViewMetadata {
        session_date,
        collected_at: None,
        source: source.to_string(),
        freshness: api_models::Freshness::Current,
    }
}

async fn options_volatility_surface(
    State(state): State<LegacyHttpPorts>,
    axum::extract::Path(ticker): axum::extract::Path<String>,
) -> Result<Json<api_models::OptionsVolatilitySurfaceResponse>, StatusCode> {
    let normalized = ticker.trim().to_ascii_uppercase();
    match state.options.volatility_surface(&normalized).await {
        Ok(surface) => {
            let snapshot_time = surface.snapshot_time;
            Ok(Json(api_models::OptionsVolatilitySurfaceResponse {
                ticker: normalized,
                snapshot_time: Some(snapshot_time),
                volatility_surface: legacy_volatility_surface(surface),
            }))
        }
        Err(PortError::NotFound(_)) => Ok(Json(api_models::OptionsVolatilitySurfaceResponse {
            ticker: normalized,
            snapshot_time: None,
            volatility_surface: api_models::DataState::Unavailable,
        })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn legacy_volatility_surface(
    surface: crate::hexagon::domain::volatility_surface::VolatilitySurface,
) -> api_models::DataState<api_models::VolatilitySurfaceOverview> {
    const LEVELS: [f64; 9] = [80.0, 85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0, 120.0];
    let observations = surface
        .points
        .iter()
        .map(|point| api_models::VolatilitySurfaceObservation {
            expiration: point.expiration,
            days: point.days_to_expiration,
            strike: point.strike,
            moneyness_percent: point.moneyness * 100.0,
            volatility_percent: point.implied_volatility * 100.0,
        })
        .collect();
    let mut expirations = surface
        .points
        .iter()
        .filter(|point| (7..=365).contains(&point.days_to_expiration))
        .map(|point| api_models::VolatilitySurfaceExpiration {
            date: point.expiration,
            days: point.days_to_expiration,
        })
        .collect::<Vec<_>>();
    expirations.sort_by_key(|expiration| expiration.date);
    expirations.dedup_by_key(|expiration| expiration.date);
    expirations.truncate(10);

    let mut points = Vec::new();
    for expiration in &expirations {
        for (level_index, level) in LEVELS.iter().enumerate() {
            let Some(candidate) = surface
                .points
                .iter()
                .filter(|point| point.expiration == expiration.date)
                .min_by(|left, right| {
                    (left.moneyness * 100.0 - level)
                        .abs()
                        .total_cmp(&(right.moneyness * 100.0 - level).abs())
                })
            else {
                continue;
            };
            let actual_moneyness = candidate.moneyness * 100.0;
            let nearest_level = LEVELS
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    (actual_moneyness - **left)
                        .abs()
                        .total_cmp(&(actual_moneyness - **right).abs())
                })
                .map(|(index, _)| index);
            if nearest_level == Some(level_index) && (actual_moneyness - level).abs() <= 3.0 {
                points.push(api_models::VolatilitySurfacePoint {
                    expiration: candidate.expiration,
                    days: candidate.days_to_expiration,
                    strike: candidate.strike,
                    moneyness_percent: *level,
                    volatility_percent: candidate.implied_volatility * 100.0,
                });
            }
        }
    }
    if points.is_empty() {
        return api_models::DataState::Unavailable;
    }
    api_models::DataState::Available(api_models::VolatilitySurfaceOverview {
        metadata: option_metadata(surface.snapshot_time.date_naive(), "market data"),
        reference_price: surface.reference_price,
        expirations,
        moneyness_levels: LEVELS.to_vec(),
        points,
        observations,
    })
}

async fn options_intraday(
    State(state): State<LegacyHttpPorts>,
    Path(ticker): Path<String>,
) -> Result<Json<api_models::OptionsIntradayResponse>, StatusCode> {
    let market = state
        .intraday_options
        .intraday_options(&ticker)
        .await
        .map_err(port_status)?;
    let normalized = ticker.trim().to_ascii_uppercase();
    let catalog = crate::driving_adapters::http::legacy_simulation_views::catalog(
        &normalized,
        &market.snapshot,
        market.spot,
    );
    let volatility_surface =
        crate::hexagon::domain::volatility_surface::VolatilitySurface::from_snapshot(
            &market.snapshot,
            market.spot,
        )
        .map(legacy_volatility_surface)
        .unwrap_or(api_models::DataState::Unavailable);
    Ok(Json(api_models::OptionsIntradayResponse {
        ticker: normalized,
        snapshot_time: market.snapshot.timestamp_utc,
        catalog,
        volatility_surface,
    }))
}

fn asset_summary(tracked: &TrackedTicker) -> AssetSummary {
    let (name, kind) = match tracked.ticker.as_str() {
        "AAPL" => ("Apple Inc.", AssetKind::Equity),
        "GOOGL" => ("Alphabet Inc.", AssetKind::Equity),
        "IBM" => ("IBM", AssetKind::Equity),
        "JPM" => ("JPMorgan Chase & Co.", AssetKind::Equity),
        "MSFT" => ("Microsoft Corp.", AssetKind::Equity),
        "SPX" => ("S&P 500 Index", AssetKind::Index),
        "SPY" => ("SPDR S&P 500 ETF Trust", AssetKind::Etf),
        _ => (tracked.ticker.as_str(), AssetKind::Equity),
    };
    AssetSummary {
        ticker: tracked.ticker.clone(),
        name: name.to_string(),
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_spy_as_an_etf() {
        let tracked = TrackedTicker {
            ticker: "SPY".to_string(),
            source: crate::hexagon::domain::tracked_ticker::TrackedTickerSource::System,
            active: true,
            historical_prices: false,
            option_snapshots: true,
        };

        let summary = asset_summary(&tracked);

        assert_eq!(summary.kind, AssetKind::Etf);
        assert_eq!(summary.name, "SPDR S&P 500 ETF Trust");
    }

    #[test]
    fn classifies_spx_as_an_index() {
        let tracked = TrackedTicker {
            ticker: "SPX".to_string(),
            source: crate::hexagon::domain::tracked_ticker::TrackedTickerSource::System,
            active: true,
            historical_prices: false,
            option_snapshots: true,
        };

        let summary = asset_summary(&tracked);

        assert_eq!(summary.kind, AssetKind::Index);
        assert_eq!(summary.name, "S&P 500 Index");
    }

    #[test]
    fn maps_domain_volatility_surface_to_legacy_percent_units() {
        let expiration = chrono::NaiveDate::from_ymd_opt(2026, 9, 18).expect("valid date");
        let surface = crate::hexagon::domain::volatility_surface::VolatilitySurface {
            ticker: "SPY".to_string(),
            snapshot_time: expiration
                .and_hms_opt(20, 0, 0)
                .expect("valid time")
                .and_utc(),
            reference_price: 100.0,
            points: vec![
                crate::hexagon::domain::volatility_surface::VolatilitySurfacePoint {
                    expiration,
                    days_to_expiration: 30,
                    strike: 100.0,
                    moneyness: 1.0,
                    option_type: crate::hexagon::domain::options::OptionType::Call,
                    implied_volatility: 0.25,
                },
            ],
        };

        let api_models::DataState::Available(mapped) = legacy_volatility_surface(surface) else {
            panic!("surface must be available");
        };
        assert_eq!(mapped.moneyness_levels[4], 100.0);
        assert_eq!(mapped.points[0].volatility_percent, 25.0);
    }

    #[test]
    fn maps_market_history_to_legacy_price_change() {
        let quote = |day, close| crate::hexagon::domain::market_history::DailyQuote {
            timestamp: chrono::NaiveDate::from_ymd_opt(2026, 8, day)
                .expect("valid date")
                .and_hms_opt(20, 0, 0)
                .expect("valid time")
                .and_utc(),
            open: Some(close),
            high: Some(close),
            low: Some(close),
            close: Some(close),
            adjusted_close: Some(close),
            volume: Some(100),
        };
        let history = crate::hexagon::domain::market_history::MarketHistory {
            ticker: "TEST".to_string(),
            currency: Some("USD".to_string()),
            exchange_timezone: None,
            daily_quotes: vec![quote(3, 100.0), quote(4, 105.0)],
            dividends: Vec::new(),
            splits: Vec::new(),
        };

        let mapped = legacy_asset_price("TEST".to_string(), &history);
        let api_models::DataState::Available(price) = mapped.price else {
            panic!("price must be available");
        };
        assert!((price.daily_change_pct.expect("change must exist") - 5.0).abs() < 1e-12);
    }
}
