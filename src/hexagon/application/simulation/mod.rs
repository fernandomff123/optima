//! Strategy-simulation use cases.

use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use std::collections::BTreeMap;

use crate::hexagon::{
    PortError, PortResult,
    domain::simulation::{
        BlackScholesSimulator, ExerciseStyle, MarketState, OptionContract, PricingConfig,
        PricingModel, ScenarioGrid, SimulationCurve, SimulationLeg, SimulationLegSelection,
        SimulationRequest, SimulationResult, SimulationScenario, SimulationScenarioPoint,
        SimulationStrategyKind, SimulationTradeSide, SimulationWarning, Strategy, StrategyLeg,
        StrategySimulator,
    },
    driving_ports::for_simulating_strategies::{
        ForSimulatingStrategies, ScenarioGridRequest, SimulateScenario,
    },
};

/// Executes deterministic strategy simulations without infrastructure concerns.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimulationApplication;

/// Strategy selected and validated against one option-chain snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedStrategy {
    pub id: String,
    pub label: String,
    pub expiration: NaiveDate,
    pub strikes: Vec<f64>,
    pub strategy: Strategy,
}

const DEFAULT_VOLATILITY_SHIFTS: [f64; 3] = [-0.10, 0.0, 0.10];
const SPOT_SCENARIO_COUNT: usize = 321;

/// Selects contracts and constructs the strategy requested by the driving actor.
pub fn prepare_strategy(
    ticker: &str,
    snapshot: &crate::hexagon::domain::options::Snapshot,
    spot: f64,
    strategy_kind: SimulationStrategyKind,
    requested_legs: &[SimulationLegSelection],
) -> PortResult<PreparedStrategy> {
    let valuation_date = snapshot.timestamp_utc.date_naive();
    let default_expiration = snapshot
        .contratos
        .iter()
        .map(|contract| contract.expiration)
        .filter(|expiration| *expiration > valuation_date)
        .min_by_key(|expiration| ((*expiration - valuation_date).num_days() - 30).abs())
        .ok_or_else(|| PortError::NotFound("expiração do ativo indisponível".to_string()))?;
    let custom_contracts = requested_legs
        .iter()
        .map(|requested| {
            if requested.quantity == 0
                || !requested.entry_price.is_finite()
                || requested.entry_price < 0.0
            {
                return Err(PortError::InvalidRequest(
                    "perna da estratégia inválida".to_string(),
                ));
            }
            let contract = snapshot
                .contratos
                .iter()
                .find(|contract| contract.occ_symbol == requested.occ_symbol)
                .ok_or_else(|| {
                    PortError::InvalidRequest("contrato não existe no snapshot".to_string())
                })?;
            Ok((requested, contract))
        })
        .collect::<PortResult<Vec<_>>>()?;
    let expiration = custom_contracts
        .iter()
        .map(|(_, contract)| contract.expiration)
        .min()
        .unwrap_or(default_expiration);
    let mut paired = BTreeMap::new();
    for contract in snapshot
        .contratos
        .iter()
        .filter(|contract| contract.expiration == expiration)
    {
        let pair = paired
            .entry((contract.strike * 1000.0).round() as i64)
            .or_insert((None, None));
        match contract.option_type {
            crate::hexagon::domain::options::OptionType::Call => pair.0 = Some(contract),
            crate::hexagon::domain::options::OptionType::Put => pair.1 = Some(contract),
        }
    }
    let (call, put) = paired
        .values()
        .filter_map(|(call, put)| call.zip(*put))
        .min_by(|(left, _), (right, _)| {
            (left.strike - spot)
                .abs()
                .total_cmp(&(right.strike - spot).abs())
        })
        .ok_or_else(|| PortError::NotFound("par ATM do ativo indisponível".to_string()))?;
    let make_leg = |contract: &crate::hexagon::domain::options::ContratoOpcao,
                    quantity: i32,
                    entry_price: f64| StrategyLeg {
        contract: OptionContract {
            symbol: contract.occ_symbol.clone(),
            option_type: contract.option_type,
            exercise_style: ExerciseStyle::European,
            strike: contract.strike,
            expiration: contract.expiration,
        },
        quantity,
        multiplier: 100,
        entry_price,
        entry_volatility: contract.implied_volatility,
        fees: 0.0,
    };
    let short_call = paired
        .values()
        .filter_map(|(candidate, _)| *candidate)
        .filter(|candidate| candidate.strike > call.strike)
        .min_by(|left, right| {
            (left.strike - (call.strike + spot * 0.01))
                .abs()
                .total_cmp(&(right.strike - (call.strike + spot * 0.01)).abs())
        })
        .ok_or_else(|| PortError::NotFound("call superior do ativo indisponível".to_string()))?;
    let (id, label, legs) = if !custom_contracts.is_empty() {
        (
            "custom",
            "Estratégia personalizada",
            custom_contracts
                .iter()
                .map(|(requested, contract)| {
                    let direction = match requested.side {
                        SimulationTradeSide::Buy => 1,
                        SimulationTradeSide::Sell => -1,
                    };
                    make_leg(
                        contract,
                        direction * requested.quantity as i32,
                        requested.entry_price,
                    )
                })
                .collect(),
        )
    } else {
        match strategy_kind {
            SimulationStrategyKind::Straddle => (
                "atm-straddle",
                "Straddle ATM",
                vec![make_leg(call, 1, call.ask), make_leg(put, 1, put.ask)],
            ),
            SimulationStrategyKind::BullCallSpread => (
                "bull-call-spread",
                "Bull call spread",
                vec![
                    make_leg(call, 1, call.ask),
                    make_leg(short_call, -1, short_call.bid),
                ],
            ),
            SimulationStrategyKind::Custom => {
                return Err(PortError::InvalidRequest(
                    "adicione pelo menos uma perna".to_string(),
                ));
            }
        }
    };
    let mut strikes = legs
        .iter()
        .map(|leg| leg.contract.strike)
        .collect::<Vec<_>>();
    strikes.sort_by(|left, right| left.total_cmp(right));
    strikes.dedup_by(|left, right| (*left - *right).abs() < 1.0e-9);
    Ok(PreparedStrategy {
        id: id.to_string(),
        label: label.to_string(),
        expiration,
        strikes,
        strategy: Strategy {
            id: Some(id.to_string()),
            root: ticker.to_string(),
            legs,
        },
    })
}

/// Normalizes scenario dates while enforcing the strategy lifetime.
///
/// Empty input selects a small representative set between valuation and
/// expiration. Duplicate dates are intentionally collapsed because running
/// the same scenario twice would only duplicate output.
pub fn scenario_dates(
    valuation_date: NaiveDate,
    expiration: NaiveDate,
    requested: Option<&[NaiveDate]>,
) -> PortResult<Vec<NaiveDate>> {
    let defaults = [
        valuation_date,
        (valuation_date + Duration::days(7)).min(expiration),
        (valuation_date + Duration::days(14)).min(expiration),
        expiration,
    ];
    let dates = requested
        .filter(|dates| !dates.is_empty())
        .unwrap_or(&defaults);
    if dates
        .iter()
        .any(|date| *date < valuation_date || *date > expiration)
    {
        return Err(PortError::InvalidRequest(
            "data de simulação fora do intervalo da estratégia".to_string(),
        ));
    }
    Ok(dates
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect())
}

/// Normalizes volatility shocks and always includes the unchanged base case.
pub fn scenario_volatility_shifts(requested: &[f64]) -> PortResult<Vec<f64>> {
    let shifts = if requested.is_empty() {
        DEFAULT_VOLATILITY_SHIFTS.as_slice()
    } else {
        requested
    };
    if shifts
        .iter()
        .any(|shift| !shift.is_finite() || shift.abs() > 0.60)
    {
        return Err(PortError::InvalidRequest(
            "choque de volatilidade fora do intervalo permitido".to_string(),
        ));
    }
    let mut points = shifts
        .iter()
        .map(|shift| (shift * 100.0).round() as i32)
        .collect::<std::collections::BTreeSet<_>>();
    points.insert(0);
    Ok(points
        .into_iter()
        .map(|point| point as f64 / 100.0)
        .collect())
}

/// Keeps only the vertices required to represent the piecewise-linear payoff.
pub fn expiration_payoff_segments(
    points: &[SimulationScenarioPoint],
    strikes: &[f64],
) -> Vec<SimulationScenarioPoint> {
    let Some(first) = points.first() else {
        return Vec::new();
    };
    let Some(last) = points.last() else {
        return Vec::new();
    };
    let mut segments = vec![first.clone()];
    for strike in strikes {
        if let Some(vertex) = points.iter().min_by(|left, right| {
            (left.spot - strike)
                .abs()
                .total_cmp(&(right.spot - strike).abs())
        }) && segments.last() != Some(vertex)
        {
            segments.push(vertex.clone());
        }
    }
    if segments.last() != Some(last) {
        segments.push(last.clone());
    }
    segments
}

/// Finds zero crossings by linear interpolation between adjacent scenario points.
pub fn break_even_points(points: &[SimulationScenarioPoint]) -> Vec<f64> {
    points
        .windows(2)
        .filter_map(|pair| {
            let (left, right) = (&pair[0], &pair[1]);
            if left.pnl == 0.0 {
                Some(left.spot)
            } else if left.pnl.signum() != right.pnl.signum() {
                Some(left.spot + (right.spot - left.spot) * -left.pnl / (right.pnl - left.pnl))
            } else {
                None
            }
        })
        .collect()
}

fn execute_scenario(command: SimulateScenario) -> PortResult<SimulationScenario> {
    let valuation_date = command.snapshot.timestamp_utc.date_naive();
    let prepared = prepare_strategy(
        &command.ticker,
        &command.snapshot,
        command.spot,
        command.strategy_kind,
        &command.legs,
    )?;
    let expiration = prepared.expiration;
    let single_expiration = prepared
        .strategy
        .legs
        .iter()
        .all(|leg| leg.contract.expiration == expiration);
    let primary_strike = *prepared
        .strikes
        .first()
        .ok_or_else(|| PortError::InvalidRequest("estratégia sem strikes".to_string()))?;
    let upper_strike = prepared.strikes.get(1).copied();
    let days = (expiration - valuation_date).num_days() as f64;
    let risk_free_rate =
        crate::hexagon::domain::interest_rates::BoundedCubicSpline::from_treasury_curve(
            &command.yield_curve,
        )
        .and_then(|spline| spline.continuously_compounded_rate(days))
        .map_err(invalid_simulation)?;
    let legs = prepared
        .strategy
        .legs
        .iter()
        .map(|leg| SimulationLeg {
            occ_symbol: leg.contract.symbol.clone(),
            option_type: leg.contract.option_type,
            strike: leg.contract.strike,
            expiration: leg.contract.expiration,
            side: if leg.quantity > 0 {
                SimulationTradeSide::Buy
            } else {
                SimulationTradeSide::Sell
            },
            quantity: leg.quantity.unsigned_abs(),
            entry_price: leg.entry_price,
            base_volatility: leg.entry_volatility.unwrap_or(0.2),
        })
        .collect();
    let dates = scenario_dates(
        valuation_date,
        expiration,
        command.valuation_dates.as_deref(),
    )?;
    let shifts = scenario_volatility_shifts(&command.volatility_shifts)?;
    let mut grid = ScenarioGrid::centered(command.spot, 0.2, SPOT_SCENARIO_COUNT, dates, shifts)
        .map_err(invalid_simulation)?;
    for strike in &prepared.strikes {
        grid.include_spot(*strike).map_err(invalid_simulation)?;
    }
    let result = BlackScholesSimulator
        .simulate_strategy(&SimulationRequest {
            strategy: prepared.strategy.clone(),
            market: MarketState {
                valuation_date,
                spot: command.spot,
                risk_free_rate,
                dividend_yield: 0.0,
                volatility: 0.2,
                snapshot_id: Some(command.snapshot.timestamp_utc.to_rfc3339()),
            },
            grid,
            pricing: PricingConfig {
                european_model: PricingModel::BlackScholes,
                american_model: PricingModel::Binomial { steps: 200 },
            },
        })
        .map_err(invalid_simulation)?;
    let mut curves = BTreeMap::new();
    for point in result.points {
        if single_expiration && point.valuation_date == expiration && point.volatility_shift != 0.0
        {
            continue;
        }
        let shift_points = (point.volatility_shift * 100.0).round() as i32;
        let curve = curves
            .entry((point.valuation_date, shift_points))
            .or_insert_with(|| (Vec::new(), false));
        curve.1 |= point
            .warnings
            .contains(&SimulationWarning::VolatilityFloored);
        curve.0.push(SimulationScenarioPoint {
            spot: point.spot,
            pnl: point.pnl,
            greeks: point.greeks,
        });
    }
    if single_expiration && let Some((points, _)) = curves.get_mut(&(expiration, 0)) {
        *points = expiration_payoff_segments(points, &prepared.strikes);
    }
    let break_even_points = curves
        .get(&(expiration, 0))
        .map(|(points, _)| self::break_even_points(points))
        .unwrap_or_default();
    Ok(SimulationScenario {
        ticker: command.ticker.clone(),
        strategy_kind: if command.legs.is_empty() {
            command.strategy_kind
        } else {
            SimulationStrategyKind::Custom
        },
        strategy_label: prepared.label,
        valuation_date,
        expiration,
        strike: primary_strike,
        upper_strike,
        spot: command.spot,
        model: if command.ticker == "SPX" {
            "Black–Scholes · opção europeia".to_string()
        } else {
            "Black–Scholes provisório · exercício americano ainda não modelado".to_string()
        },
        break_even_points,
        legs,
        curves: curves
            .into_iter()
            .map(
                |((date, shift_points), (points, volatility_limited))| SimulationCurve {
                    label: if date == expiration {
                        if single_expiration {
                            "Vencimento".to_string()
                        } else {
                            "1.º vencimento".to_string()
                        }
                    } else if date == valuation_date {
                        "Data inicial".to_string()
                    } else {
                        date.format("%d/%m").to_string()
                    },
                    valuation_date: date,
                    volatility_shift: shift_points as f64 / 100.0,
                    volatility_limited,
                    points,
                },
            )
            .collect(),
    })
}

#[async_trait]
impl ForSimulatingStrategies for SimulationApplication {
    async fn build_scenario_grid(&self, request: ScenarioGridRequest) -> PortResult<ScenarioGrid> {
        let mut grid = ScenarioGrid::centered(
            request.spot,
            request.range_fraction,
            request.spot_count,
            request.valuation_dates,
            request.volatility_shifts,
        )
        .map_err(invalid_simulation)?;
        for spot in request.required_spots {
            grid.include_spot(spot).map_err(invalid_simulation)?;
        }
        Ok(grid)
    }

    async fn simulate_strategy(&self, request: SimulationRequest) -> PortResult<SimulationResult> {
        BlackScholesSimulator
            .simulate_strategy(&request)
            .map_err(invalid_simulation)
    }

    async fn simulate_scenario(&self, command: SimulateScenario) -> PortResult<SimulationScenario> {
        execute_scenario(command)
    }
}

fn invalid_simulation(error: impl std::fmt::Display) -> PortError {
    PortError::InvalidRequest(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::domain::options::{ContratoOpcao, OptionType, Snapshot};
    use chrono::{TimeZone, Utc};

    fn point(spot: f64, pnl: f64) -> SimulationScenarioPoint {
        SimulationScenarioPoint {
            spot,
            pnl,
            greeks: crate::hexagon::domain::simulation::Greeks::default(),
        }
    }

    fn contract(symbol: &str, option_type: OptionType, strike: f64) -> ContratoOpcao {
        ContratoOpcao {
            occ_symbol: symbol.to_string(),
            option_type,
            strike,
            expiration: NaiveDate::from_ymd_opt(2026, 8, 21).expect("valid fixture date"),
            bid: 4.0,
            ask: 4.5,
            mid: 4.25,
            spread: 0.5,
            volume: 10.0,
            open_interest: 100.0,
            delta: 0.5,
            gamma: 0.01,
            vega: 0.1,
            theta: -0.1,
            rho: 0.01,
            theo: 4.25,
            implied_volatility: Some(0.2),
        }
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            ticker: "SPX".to_string(),
            timestamp_utc: Utc
                .with_ymd_and_hms(2026, 7, 17, 20, 0, 0)
                .single()
                .expect("valid fixture timestamp"),
            contratos: vec![
                contract("SPX-C100", OptionType::Call, 100.0),
                contract("SPX-P100", OptionType::Put, 100.0),
                contract("SPX-C105", OptionType::Call, 105.0),
                contract("SPX-P105", OptionType::Put, 105.0),
            ],
            chains: Vec::new(),
        }
    }

    #[test]
    fn prepares_an_atm_straddle_from_the_snapshot() {
        let prepared = prepare_strategy(
            "SPX",
            &snapshot(),
            101.0,
            SimulationStrategyKind::Straddle,
            &[],
        )
        .expect("valid strategy");

        assert_eq!(prepared.id, "atm-straddle");
        assert_eq!(prepared.strikes, vec![100.0]);
        assert_eq!(prepared.strategy.legs.len(), 2);
        assert!(prepared.strategy.legs.iter().all(|leg| leg.quantity == 1));
    }

    #[test]
    fn rejects_a_custom_strategy_without_legs() {
        let error = prepare_strategy(
            "SPX",
            &snapshot(),
            101.0,
            SimulationStrategyKind::Custom,
            &[],
        )
        .expect_err("custom strategy must contain legs");

        assert!(matches!(error, PortError::InvalidRequest(_)));
    }

    #[test]
    fn interpolates_both_expiration_break_even_points() {
        let points = [
            point(80.0, 10.0),
            point(90.0, -10.0),
            point(110.0, -10.0),
            point(120.0, 10.0),
        ];

        assert_eq!(break_even_points(&points), vec![85.0, 115.0]);
    }

    #[test]
    fn reduces_expiration_payoff_to_two_straight_segments() {
        let points = [
            point(80.0, 10.0),
            point(90.0, -10.0),
            point(100.0, -20.0),
            point(110.0, -10.0),
            point(120.0, 10.0),
        ];

        assert_eq!(
            expiration_payoff_segments(&points, &[100.0]),
            vec![points[0].clone(), points[2].clone(), points[4].clone()]
        );
    }

    #[test]
    fn preserves_both_vertical_spread_strikes_at_expiration() {
        let points = [
            point(90.0, -5.0),
            point(100.0, -5.0),
            point(110.0, 5.0),
            point(120.0, 5.0),
        ];

        assert_eq!(expiration_payoff_segments(&points, &[100.0, 110.0]), points);
    }

    #[test]
    fn normalizes_requested_scenario_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 17).expect("valid fixture date");
        let expiration = NaiveDate::from_ymd_opt(2026, 8, 14).expect("valid fixture date");
        let requested = [expiration, start, expiration];

        let dates = scenario_dates(start, expiration, Some(&requested)).expect("valid dates");

        assert_eq!(dates, vec![start, expiration]);
    }

    #[test]
    fn rejects_scenario_dates_after_expiration() {
        let start = NaiveDate::from_ymd_opt(2026, 7, 17).expect("valid fixture date");
        let expiration = NaiveDate::from_ymd_opt(2026, 8, 14).expect("valid fixture date");
        let requested = [expiration + Duration::days(1)];

        assert!(scenario_dates(start, expiration, Some(&requested)).is_err());
    }

    #[test]
    fn normalizes_volatility_shifts_and_keeps_the_base_case() {
        let shifts =
            scenario_volatility_shifts(&[0.20, -0.20, 0.20]).expect("valid volatility shifts");

        assert_eq!(shifts, vec![-0.20, 0.0, 0.20]);
    }

    #[test]
    fn rejects_excessive_volatility_shifts() {
        assert!(scenario_volatility_shifts(&[0.61]).is_err());
    }
}
