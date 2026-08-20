//! Explicit wire/domain mappings for the canonical public API.

use crate::hexagon::{
    PortError,
    domain::{
        market_history as mh, options as opt, portfolio as pf, saved_strategy as ss,
        simulation as sim, tracked_ticker as tt, volatility as vol, volatility_surface as vs,
    },
    driving_ports::{
        for_simulating_strategies::ScenarioGridRequest, for_synchronizing_market_data as sync,
    },
};

pub fn market_history(value: mh::MarketHistory) -> api_models::MarketHistory {
    api_models::MarketHistory {
        ticker: value.ticker,
        currency: value.currency,
        exchange_timezone: value.exchange_timezone,
        daily_quotes: value
            .daily_quotes
            .into_iter()
            .map(|v| api_models::DailyQuote {
                timestamp: v.timestamp,
                open: v.open,
                high: v.high,
                low: v.low,
                close: v.close,
                adjusted_close: v.adjusted_close,
                volume: v.volume,
            })
            .collect(),
        dividends: value
            .dividends
            .into_iter()
            .map(|v| api_models::MarketDividend {
                timestamp: v.timestamp,
                amount: v.amount,
            })
            .collect(),
        splits: value
            .splits
            .into_iter()
            .map(|v| api_models::StockSplit {
                timestamp: v.timestamp,
                numerator: v.numerator,
                denominator: v.denominator,
                ratio: v.ratio,
            })
            .collect(),
    }
}

pub fn live_price(value: crate::hexagon::domain::live_price::LivePrice) -> api_models::LivePrice {
    api_models::LivePrice {
        ticker: value.ticker,
        price: value.price,
        market_time: value.market_time,
        currency: value.currency,
        exchange: value.exchange,
        regular_session: value.regular_session,
        change: value.change,
        change_percent: value.change_percent,
        day_volume: value.day_volume,
    }
}

fn option_type(value: opt::OptionType) -> api_models::OptionType {
    match value {
        opt::OptionType::Call => api_models::OptionType::Call,
        opt::OptionType::Put => api_models::OptionType::Put,
    }
}

fn domain_option_type(value: api_models::OptionType) -> opt::OptionType {
    match value {
        api_models::OptionType::Call => opt::OptionType::Call,
        api_models::OptionType::Put => opt::OptionType::Put,
    }
}

fn option_contract(value: opt::ContratoOpcao) -> api_models::OptionContract {
    api_models::OptionContract {
        occ_symbol: value.occ_symbol,
        option_type: option_type(value.option_type),
        strike: value.strike,
        expiration: value.expiration,
        bid: value.bid,
        ask: value.ask,
        mid: value.mid,
        spread: value.spread,
        volume: value.volume,
        open_interest: value.open_interest,
        delta: value.delta,
        gamma: value.gamma,
        vega: value.vega,
        theta: value.theta,
        rho: value.rho,
        theo: value.theo,
        implied_volatility: value.implied_volatility,
    }
}

pub fn option_snapshot(value: opt::Snapshot) -> api_models::OptionSnapshot {
    api_models::OptionSnapshot {
        ticker: value.ticker,
        timestamp_utc: value.timestamp_utc,
        contratos: value.contratos.into_iter().map(option_contract).collect(),
        chains: value
            .chains
            .into_iter()
            .map(|v| api_models::OptionChain {
                root: v.root,
                contratos: v.contratos.into_iter().map(option_contract).collect(),
            })
            .collect(),
    }
}

pub fn term_structure(value: vol::TermStructure) -> api_models::TermStructure {
    api_models::TermStructure {
        ticker: value.ticker,
        snapshot_timestamp: value.snapshot_timestamp,
        treasury_date: value.treasury_date,
        points: value
            .points
            .into_iter()
            .map(|v| api_models::TermStructurePoint {
                days: v.days,
                variance: v.variance,
                volatility: v.volatility,
                source: match v.source {
                    vol::TermStructureSource::Interpolated {
                        near_expiration,
                        near_rate,
                        next_expiration,
                        next_rate,
                    } => api_models::TermStructureSource::Interpolated {
                        near_expiration,
                        near_rate,
                        next_expiration,
                        next_rate,
                    },
                    vol::TermStructureSource::Expiration {
                        expiration,
                        interest_rate,
                    } => api_models::TermStructureSource::Expiration {
                        expiration,
                        interest_rate,
                    },
                },
            })
            .collect(),
    }
}

fn surface_point(value: vs::VolatilitySurfacePoint) -> api_models::CanonicalVolatilitySurfacePoint {
    api_models::CanonicalVolatilitySurfacePoint {
        expiration: value.expiration,
        days_to_expiration: value.days_to_expiration,
        strike: value.strike,
        moneyness: value.moneyness,
        option_type: option_type(value.option_type),
        implied_volatility: value.implied_volatility,
    }
}

pub fn volatility_surface(value: vs::VolatilitySurface) -> api_models::VolatilitySurface {
    api_models::VolatilitySurface {
        ticker: value.ticker,
        snapshot_time: value.snapshot_time,
        reference_price: value.reference_price,
        points: value.points.into_iter().map(surface_point).collect(),
    }
}

pub fn volatility_skew(value: vs::VolatilitySkew) -> api_models::VolatilitySkew {
    api_models::VolatilitySkew {
        ticker: value.ticker,
        expiration: value.expiration,
        points: value.points.into_iter().map(surface_point).collect(),
    }
}

pub fn greeks(value: sim::Greeks) -> api_models::Greeks {
    api_models::Greeks {
        delta: value.delta,
        gamma: value.gamma,
        vega: value.vega,
        theta: value.theta,
        rho: value.rho,
    }
}

pub fn scenario_grid(value: sim::ScenarioGrid) -> api_models::ScenarioGrid {
    api_models::ScenarioGrid {
        spots: value.spots,
        valuation_dates: value.valuation_dates,
        volatility_shifts: value.volatility_shifts,
    }
}

pub fn scenario_grid_request(value: api_models::ScenarioGridRequest) -> ScenarioGridRequest {
    ScenarioGridRequest {
        spot: value.spot,
        range_fraction: value.range_fraction,
        spot_count: value.spot_count,
        valuation_dates: value.valuation_dates,
        volatility_shifts: value.volatility_shifts,
        required_spots: value.required_spots,
    }
}

fn pricing_model(value: sim::PricingModel) -> api_models::PricingModel {
    match value {
        sim::PricingModel::BlackScholes => api_models::PricingModel::BlackScholes,
        sim::PricingModel::Binomial { steps } => api_models::PricingModel::Binomial { steps },
    }
}

fn domain_pricing_model(value: api_models::PricingModel) -> sim::PricingModel {
    match value {
        api_models::PricingModel::BlackScholes => sim::PricingModel::BlackScholes,
        api_models::PricingModel::Binomial { steps } => sim::PricingModel::Binomial { steps },
    }
}

pub fn simulation_request(value: api_models::StrategySimulationRequest) -> sim::SimulationRequest {
    sim::SimulationRequest {
        strategy: sim::Strategy {
            id: value.strategy.id,
            root: value.strategy.root,
            legs: value
                .strategy
                .legs
                .into_iter()
                .map(|v| sim::StrategyLeg {
                    contract: sim::OptionContract {
                        symbol: v.contract.symbol,
                        option_type: domain_option_type(v.contract.option_type),
                        exercise_style: match v.contract.exercise_style {
                            api_models::ExerciseStyle::European => sim::ExerciseStyle::European,
                            api_models::ExerciseStyle::American => sim::ExerciseStyle::American,
                        },
                        strike: v.contract.strike,
                        expiration: v.contract.expiration,
                    },
                    quantity: v.quantity,
                    multiplier: v.multiplier,
                    entry_price: v.entry_price,
                    entry_volatility: v.entry_volatility,
                    fees: v.fees,
                })
                .collect(),
        },
        market: sim::MarketState {
            valuation_date: value.market.valuation_date,
            spot: value.market.spot,
            risk_free_rate: value.market.risk_free_rate,
            dividend_yield: value.market.dividend_yield,
            volatility: value.market.volatility,
            snapshot_id: value.market.snapshot_id,
        },
        grid: sim::ScenarioGrid {
            spots: value.grid.spots,
            valuation_dates: value.grid.valuation_dates,
            volatility_shifts: value.grid.volatility_shifts,
        },
        pricing: sim::PricingConfig {
            european_model: domain_pricing_model(value.pricing.european_model),
            american_model: domain_pricing_model(value.pricing.american_model),
        },
    }
}

pub fn simulation_result(value: sim::SimulationResult) -> api_models::StrategySimulationResult {
    api_models::StrategySimulationResult {
        strategy_id: value.strategy_id,
        model: pricing_model(value.model),
        points: value
            .points
            .into_iter()
            .map(|v| api_models::SimulationPoint {
                spot: v.spot,
                valuation_date: v.valuation_date,
                volatility_shift: v.volatility_shift,
                theoretical_value: v.theoretical_value,
                pnl: v.pnl,
                greeks: greeks(v.greeks),
                legs: v
                    .legs
                    .into_iter()
                    .map(|l| api_models::LegSimulationResult {
                        symbol: l.symbol,
                        theoretical_price: l.theoretical_price,
                        position_value: l.position_value,
                        pnl: l.pnl,
                        intrinsic_value: l.intrinsic_value,
                        temporal_value: l.temporal_value,
                        greeks: greeks(l.greeks),
                    })
                    .collect(),
                warnings: v
                    .warnings
                    .into_iter()
                    .map(|w| match w {
                        sim::SimulationWarning::AtOrAfterExpiration => {
                            api_models::SimulationWarning::AtOrAfterExpiration
                        }
                        sim::SimulationWarning::VolatilityFloored => {
                            api_models::SimulationWarning::VolatilityFloored
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn tracked_ticker(value: tt::TrackedTicker) -> api_models::TrackedTicker {
    api_models::TrackedTicker {
        ticker: value.ticker,
        source: match value.source {
            tt::TrackedTickerSource::System => api_models::TrackedTickerSource::System,
            tt::TrackedTickerSource::User => api_models::TrackedTickerSource::User,
        },
        active: value.active,
        historical_prices: value.historical_prices,
        option_snapshots: value.option_snapshots,
        resolution_state: match value.resolution_state {
            tt::UnderlyingResolutionState::Pending => {
                api_models::UnderlyingResolutionState::Pending
            }
            tt::UnderlyingResolutionState::Resolved => {
                api_models::UnderlyingResolutionState::Resolved
            }
            tt::UnderlyingResolutionState::Rejected => {
                api_models::UnderlyingResolutionState::Rejected
            }
        },
        validated_at: value.validated_at,
        metadata: underlying_metadata(value.metadata),
    }
}

pub fn underlying_resolution(
    value: crate::hexagon::driving_ports::for_resolving_underlyings::UnderlyingResolution,
) -> api_models::UnderlyingResolution {
    api_models::UnderlyingResolution {
        ticker: value.ticker,
        validated_at: value.validated_at,
        metadata: underlying_metadata(value.metadata),
    }
}

fn underlying_metadata(value: tt::UnderlyingMetadata) -> api_models::UnderlyingMetadata {
    api_models::UnderlyingMetadata {
        currency: value.currency,
        exchange: value.exchange,
        timezone: value.timezone,
        instrument_type: value.instrument_type,
    }
}

pub fn tracked_ticker_configuration(
    value: api_models::ConfigureTrackedTickerRequest,
) -> tt::TrackedTickerConfiguration {
    tt::TrackedTickerConfiguration {
        active: value.active,
        historical_prices: value.historical_prices,
        option_snapshots: value.option_snapshots,
    }
}

fn saved_side(value: ss::StrategySide) -> api_models::SavedStrategySide {
    match value {
        ss::StrategySide::Buy => api_models::SavedStrategySide::Buy,
        ss::StrategySide::Sell => api_models::SavedStrategySide::Sell,
    }
}

fn domain_saved_side(value: api_models::SavedStrategySide) -> ss::StrategySide {
    match value {
        api_models::SavedStrategySide::Buy => ss::StrategySide::Buy,
        api_models::SavedStrategySide::Sell => ss::StrategySide::Sell,
    }
}

pub fn saved_strategy(value: ss::SavedStrategy) -> api_models::SavedStrategy {
    api_models::SavedStrategy {
        id: value.id,
        name: value.name,
        ticker: value.ticker,
        legs: value
            .legs
            .into_iter()
            .map(|v| api_models::SavedStrategyLeg {
                occ_symbol: v.occ_symbol,
                side: saved_side(v.side),
                quantity: v.quantity,
                entry_price: v.entry_price,
            })
            .collect(),
        updated_at: value.updated_at,
    }
}

pub fn save_strategy(
    value: api_models::SaveStrategy,
) -> crate::hexagon::driving_ports::for_managing_saved_strategies::SaveStrategy {
    crate::hexagon::driving_ports::for_managing_saved_strategies::SaveStrategy {
        name: value.name,
        ticker: value.ticker,
        legs: value
            .legs
            .into_iter()
            .map(|v| ss::SavedStrategyLeg {
                occ_symbol: v.occ_symbol,
                side: domain_saved_side(v.side),
                quantity: v.quantity,
                entry_price: v.entry_price,
            })
            .collect(),
    }
}

fn currency(value: String) -> Result<pf::Currency, PortError> {
    // Currency's legacy wire representation predates constructor validation.
    // Deserialize just this private-field value so aliases keep accepting the
    // exact historical payload set; every surrounding DTO mapping is explicit.
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| PortError::InvalidRequest(error.to_string()))
}

fn domain_money(value: api_models::Money) -> Result<pf::Money, PortError> {
    Ok(pf::Money::new(value.amount, currency(value.currency)?))
}

fn money(value: pf::Money) -> api_models::Money {
    api_models::Money {
        amount: value.amount,
        currency: value.currency.code().to_string(),
    }
}

fn domain_rate(value: api_models::ExchangeRate) -> Result<pf::ExchangeRate, PortError> {
    Ok(pf::ExchangeRate {
        base: currency(value.base)?,
        quote: currency(value.quote)?,
        rate: value.rate,
        reference_date: value.reference_date,
        source: value.source,
    })
}

fn rate(value: pf::ExchangeRate) -> api_models::ExchangeRate {
    api_models::ExchangeRate {
        base: value.base.code().to_string(),
        quote: value.quote.code().to_string(),
        rate: value.rate,
        reference_date: value.reference_date,
        source: value.source,
    }
}

fn domain_instrument(value: api_models::Instrument) -> pf::Instrument {
    match value {
        api_models::Instrument::Equity { ticker } => pf::Instrument::Equity { ticker },
        api_models::Instrument::Option { occ_symbol } => pf::Instrument::Option { occ_symbol },
    }
}

fn instrument(value: pf::Instrument) -> api_models::Instrument {
    match value {
        pf::Instrument::Equity { ticker } => api_models::Instrument::Equity { ticker },
        pf::Instrument::Option { occ_symbol } => api_models::Instrument::Option { occ_symbol },
    }
}

pub fn cash_movement(value: api_models::CashMovement) -> Result<pf::CashMovement, PortError> {
    Ok(pf::CashMovement {
        id: value.id,
        occurred_at: value.occurred_at,
        kind: match value.kind {
            api_models::CashMovementKind::Deposit => pf::CashMovementKind::Deposit,
            api_models::CashMovementKind::Withdrawal => pf::CashMovementKind::Withdrawal,
        },
        amount: domain_money(value.amount)?,
    })
}

pub fn trade(value: api_models::Trade) -> Result<pf::Trade, PortError> {
    Ok(pf::Trade {
        id: value.id,
        instrument: domain_instrument(value.instrument),
        side: match value.side {
            api_models::TradeSide::Buy => pf::TradeSide::Buy,
            api_models::TradeSide::Sell => pf::TradeSide::Sell,
        },
        executed_at: value.executed_at,
        quantity: value.quantity,
        unit_price: domain_money(value.unit_price)?,
        fees: value
            .fees
            .into_iter()
            .map(domain_money)
            .collect::<Result<_, _>>()?,
        settlement_rate_to_eur: value.settlement_rate_to_eur.map(domain_rate).transpose()?,
        tax_rate_to_eur: value.tax_rate_to_eur.map(domain_rate).transpose()?,
    })
}

pub fn currency_exchange(
    value: api_models::CurrencyExchange,
) -> Result<pf::CurrencyExchange, PortError> {
    Ok(pf::CurrencyExchange {
        id: value.id,
        occurred_at: value.occurred_at,
        sold: domain_money(value.sold)?,
        bought: domain_money(value.bought)?,
        rate: domain_rate(value.rate)?,
    })
}

pub fn position(value: pf::Position) -> api_models::Position {
    api_models::Position {
        instrument: instrument(value.instrument),
        quantity: value.quantity,
    }
}

fn event_trade(value: pf::Trade) -> api_models::Trade {
    api_models::Trade {
        id: value.id,
        instrument: instrument(value.instrument),
        side: match value.side {
            pf::TradeSide::Buy => api_models::TradeSide::Buy,
            pf::TradeSide::Sell => api_models::TradeSide::Sell,
        },
        executed_at: value.executed_at,
        quantity: value.quantity,
        unit_price: money(value.unit_price),
        fees: value.fees.into_iter().map(money).collect(),
        settlement_rate_to_eur: value.settlement_rate_to_eur.map(rate),
        tax_rate_to_eur: value.tax_rate_to_eur.map(rate),
    }
}

pub fn portfolio_event(value: pf::PortfolioEvent) -> api_models::PortfolioEvent {
    match value {
        pf::PortfolioEvent::Trade(v) => api_models::PortfolioEvent::Trade(event_trade(v)),
        pf::PortfolioEvent::CashMovement(v) => {
            api_models::PortfolioEvent::CashMovement(api_models::CashMovement {
                id: v.id,
                occurred_at: v.occurred_at,
                kind: match v.kind {
                    pf::CashMovementKind::Deposit => api_models::CashMovementKind::Deposit,
                    pf::CashMovementKind::Withdrawal => api_models::CashMovementKind::Withdrawal,
                },
                amount: money(v.amount),
            })
        }
        pf::PortfolioEvent::CurrencyExchange(v) => {
            api_models::PortfolioEvent::CurrencyExchange(api_models::CurrencyExchange {
                id: v.id,
                occurred_at: v.occurred_at,
                sold: money(v.sold),
                bought: money(v.bought),
                rate: rate(v.rate),
            })
        }
        pf::PortfolioEvent::Dividend(v) => {
            api_models::PortfolioEvent::Dividend(api_models::PortfolioDividend {
                id: v.id,
                instrument: instrument(v.instrument),
                paid_at: v.paid_at,
                gross: money(v.gross),
                withholding_tax: money(v.withholding_tax),
                tax_rate_to_eur: v.tax_rate_to_eur.map(rate),
            })
        }
    }
}

pub fn synchronization_report(
    value: sync::SynchronizationReport,
) -> api_models::SynchronizationReport {
    api_models::SynchronizationReport {
        items_obtained: value.items_obtained,
        items_stored: value.items_stored,
    }
}

pub fn tracked_synchronization_report(
    value: sync::TrackedTickersSynchronizationReport,
) -> api_models::TrackedTickersSynchronizationReport {
    api_models::TrackedTickersSynchronizationReport {
        tickers: value.tickers,
        items_obtained: value.items_obtained,
        items_stored: value.items_stored,
        failures: value
            .failures
            .into_iter()
            .map(|v| api_models::SynchronizationFailure {
                ticker: v.ticker,
                operation: v.operation,
                error: v.error,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use rust_decimal::Decimal;
    use serde::Serialize;

    fn assert_same_json<T: Serialize, U: Serialize>(legacy: &T, dto: &U) {
        assert_eq!(
            serde_json::to_value(legacy).expect("legacy value must serialize"),
            serde_json::to_value(dto).expect("DTO must serialize")
        );
    }

    #[test]
    fn market_option_surface_and_greeks_keep_the_legacy_json() {
        let timestamp = Utc.with_ymd_and_hms(2026, 8, 13, 20, 0, 0).unwrap();
        let history = mh::MarketHistory {
            ticker: "SPY".into(),
            currency: None,
            exchange_timezone: Some("America/New_York".into()),
            daily_quotes: vec![mh::DailyQuote {
                timestamp,
                open: Some(1.0),
                high: None,
                low: Some(0.5),
                close: Some(1.25),
                adjusted_close: None,
                volume: Some(42),
            }],
            dividends: vec![mh::Dividend {
                timestamp,
                amount: 0.25,
            }],
            splits: vec![mh::StockSplit {
                timestamp,
                numerator: 2.0,
                denominator: 1.0,
                ratio: "2:1".into(),
            }],
        };
        assert_same_json(&history, &market_history(history.clone()));

        let contract = opt::ContratoOpcao {
            occ_symbol: "SPY260821C00500000".into(),
            option_type: opt::OptionType::Call,
            strike: 500.0,
            expiration: NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
            bid: 1.0,
            ask: 1.2,
            mid: 1.1,
            spread: 0.2,
            volume: 3.0,
            open_interest: 4.0,
            delta: 0.5,
            gamma: 0.02,
            vega: 0.1,
            theta: -0.03,
            rho: 0.01,
            theo: 1.15,
            implied_volatility: None,
            contract_specification: None,
        };
        let snapshot = opt::Snapshot {
            ticker: "SPY".into(),
            timestamp_utc: timestamp,
            contratos: vec![contract.clone()],
            chains: vec![opt::OptionChain {
                root: "SPY".into(),
                contratos: vec![contract],
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: Default::default(),
        };
        assert_same_json(&snapshot, &option_snapshot(snapshot.clone()));

        let surface = vs::VolatilitySurface {
            ticker: "SPY".into(),
            snapshot_time: timestamp,
            reference_price: 500.0,
            points: vec![vs::VolatilitySurfacePoint {
                expiration: NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
                days_to_expiration: 8,
                strike: 500.0,
                moneyness: 1.0,
                option_type: opt::OptionType::Put,
                implied_volatility: 0.2,
            }],
        };
        assert_same_json(&surface, &volatility_surface(surface.clone()));
        let legacy_greeks = sim::Greeks {
            delta: 1.0,
            gamma: 2.0,
            vega: 3.0,
            theta: 4.0,
            rho: 5.0,
        };
        assert_same_json(&legacy_greeks, &greeks(legacy_greeks));
    }

    #[test]
    fn portfolio_money_instruments_and_transactions_keep_the_legacy_json() {
        let timestamp = Utc.with_ymd_and_hms(2026, 8, 13, 20, 0, 0).unwrap();
        let money = pf::Money::new(Decimal::new(12345, 2), pf::Currency::eur());
        let legacy_trade = pf::Trade {
            id: "trade-1".into(),
            instrument: pf::Instrument::Option {
                occ_symbol: "SPY260821C00500000".into(),
            },
            side: pf::TradeSide::Buy,
            executed_at: timestamp,
            quantity: Decimal::new(2, 0),
            unit_price: money.clone(),
            fees: vec![pf::Money::new(Decimal::new(150, 2), pf::Currency::eur())],
            settlement_rate_to_eur: None,
            tax_rate_to_eur: Some(pf::ExchangeRate {
                base: pf::Currency::eur(),
                quote: pf::Currency::eur(),
                rate: Decimal::ONE,
                reference_date: timestamp.date_naive(),
                source: "ECB".into(),
            }),
        };
        let event = pf::PortfolioEvent::Trade(legacy_trade.clone());
        assert_same_json(&event, &portfolio_event(event.clone()));
        let wire: api_models::PortfolioEvent =
            serde_json::from_value(serde_json::to_value(&event).expect("event must serialize"))
                .expect("legacy JSON must deserialize as DTO");
        let api_models::PortfolioEvent::Trade(wire_trade) = wire else {
            panic!("trade expected")
        };
        assert_same_json(
            &legacy_trade,
            &trade(wire_trade).expect("wire trade must map"),
        );
    }

    #[test]
    fn saved_strategy_tracked_ticker_and_synchronization_keep_legacy_json() {
        let saved = ss::SavedStrategy {
            id: 7,
            name: "Long call".into(),
            ticker: "SPY".into(),
            legs: vec![ss::SavedStrategyLeg {
                occ_symbol: "SPY260821C00500000".into(),
                side: ss::StrategySide::Buy,
                quantity: 1,
                entry_price: 2.5,
            }],
            updated_at: Utc.with_ymd_and_hms(2026, 8, 13, 20, 0, 0).unwrap(),
        };
        assert_same_json(&saved, &saved_strategy(saved.clone()));
        let ticker = tt::TrackedTicker {
            ticker: "SPY".into(),
            source: tt::TrackedTickerSource::System,
            active: true,
            historical_prices: false,
            option_snapshots: true,
            resolution_state: tt::UnderlyingResolutionState::Resolved,
            validated_at: None,
            metadata: tt::UnderlyingMetadata::default(),
        };
        assert_same_json(&ticker, &tracked_ticker(ticker.clone()));
        let report = sync::TrackedTickersSynchronizationReport {
            tickers: 2,
            items_obtained: 3,
            items_stored: 4,
            failures: vec![sync::SynchronizationFailure {
                ticker: "VIX".into(),
                operation: "history".into(),
                error: "missing".into(),
            }],
        };
        assert_same_json(&report, &tracked_synchronization_report(report.clone()));
    }

    #[test]
    fn simulation_results_warnings_and_nested_numbers_keep_legacy_json() {
        let result = sim::SimulationResult {
            strategy_id: None,
            model: sim::PricingModel::Binomial { steps: 25 },
            points: vec![sim::SimulationPoint {
                spot: 500.25,
                valuation_date: NaiveDate::from_ymd_opt(2026, 8, 13).unwrap(),
                volatility_shift: -0.1,
                theoretical_value: 12.5,
                pnl: -1.25,
                greeks: sim::Greeks {
                    delta: 0.5,
                    gamma: 0.01,
                    vega: 0.2,
                    theta: -0.03,
                    rho: 0.04,
                },
                legs: vec![sim::LegSimulationResult {
                    symbol: "contract".into(),
                    theoretical_price: 1.0,
                    position_value: 100.0,
                    pnl: -5.0,
                    intrinsic_value: 25.0,
                    temporal_value: 75.0,
                    greeks: sim::Greeks::default(),
                }],
                warnings: vec![
                    sim::SimulationWarning::AtOrAfterExpiration,
                    sim::SimulationWarning::VolatilityFloored,
                ],
            }],
        };
        assert_same_json(&result, &simulation_result(result.clone()));
    }
}
