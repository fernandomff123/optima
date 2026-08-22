use crate::hexagon::domain::options::{OptionType, Snapshot};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{error::Error, f64::consts::PI, fmt};

const DAYS_PER_YEAR: f64 = 365.0;
const MIN_VOLATILITY: f64 = 1.0e-6;

/// Current market inputs required to prepare an intraday simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntradaySimulationMarket {
    pub snapshot: Snapshot,
    pub spot: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationStrategyKind {
    Straddle,
    BullCallSpread,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationTradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationLegSelection {
    pub occ_symbol: String,
    pub side: SimulationTradeSide,
    pub quantity: u32,
    pub entry_price: f64,
}

/// Technology-neutral result prepared for a strategy-scenario conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationScenario {
    pub ticker: String,
    pub strategy_kind: SimulationStrategyKind,
    pub strategy_label: String,
    pub valuation_date: NaiveDate,
    pub expiration: NaiveDate,
    pub strike: f64,
    pub upper_strike: Option<f64>,
    pub spot: f64,
    pub model: String,
    pub break_even_points: Vec<f64>,
    pub curves: Vec<SimulationCurve>,
    pub legs: Vec<SimulationLeg>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationLeg {
    pub occ_symbol: String,
    pub option_type: OptionType,
    pub strike: f64,
    pub expiration: NaiveDate,
    pub side: SimulationTradeSide,
    pub quantity: u32,
    pub entry_price: f64,
    pub base_volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationCurve {
    pub label: String,
    pub valuation_date: NaiveDate,
    pub volatility_shift: f64,
    pub volatility_limited: bool,
    pub points: Vec<SimulationScenarioPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationScenarioPoint {
    pub spot: f64,
    pub pnl: f64,
    pub greeks: Greeks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExerciseStyle {
    European,
    American,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionContract {
    pub symbol: String,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub strike: f64,
    pub expiration: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyLeg {
    pub contract: OptionContract,
    pub quantity: i32,
    pub multiplier: u32,
    pub entry_price: f64,
    pub entry_volatility: Option<f64>,
    pub fees: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Strategy {
    pub id: Option<String>,
    pub root: String,
    pub legs: Vec<StrategyLeg>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketState {
    pub valuation_date: NaiveDate,
    pub spot: f64,
    pub risk_free_rate: f64,
    pub dividend_yield: f64,
    pub volatility: f64,
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioGrid {
    pub spots: Vec<f64>,
    pub valuation_dates: Vec<NaiveDate>,
    pub volatility_shifts: Vec<f64>,
}

impl ScenarioGrid {
    pub fn centered(
        spot: f64,
        range_fraction: f64,
        spot_count: usize,
        valuation_dates: Vec<NaiveDate>,
        volatility_shifts: Vec<f64>,
    ) -> Result<Self, SimulationError> {
        if spot <= 0.0 || !(0.0..1.0).contains(&range_fraction) || spot_count < 2 {
            return Err(SimulationError::InvalidGrid);
        }
        let minimum = spot * (1.0 - range_fraction);
        let step = 2.0 * spot * range_fraction / (spot_count - 1) as f64;
        Ok(Self {
            spots: (0..spot_count)
                .map(|index| minimum + step * index as f64)
                .collect(),
            valuation_dates,
            volatility_shifts,
        })
    }

    pub fn include_spot(&mut self, spot: f64) -> Result<(), SimulationError> {
        if spot <= 0.0 || !spot.is_finite() {
            return Err(SimulationError::InvalidGrid);
        }
        if !self.spots.contains(&spot) {
            self.spots.push(spot);
            self.spots.sort_by(|left, right| left.total_cmp(right));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingModel {
    BlackScholes,
    Binomial { steps: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingConfig {
    pub european_model: PricingModel,
    pub american_model: PricingModel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub strategy: Strategy,
    pub market: MarketState,
    pub grid: ScenarioGrid,
    pub pricing: PricingConfig,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Greeks {
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

impl Greeks {
    fn scaled(self, scale: f64) -> Self {
        Self {
            delta: self.delta * scale,
            gamma: self.gamma * scale,
            vega: self.vega * scale,
            theta: self.theta * scale,
            rho: self.rho * scale,
        }
    }

    fn add_assign(&mut self, other: Self) {
        self.delta += other.delta;
        self.gamma += other.gamma;
        self.vega += other.vega;
        self.theta += other.theta;
        self.rho += other.rho;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegSimulationResult {
    pub symbol: String,
    pub theoretical_price: f64,
    pub position_value: f64,
    pub pnl: f64,
    pub intrinsic_value: f64,
    pub temporal_value: f64,
    pub greeks: Greeks,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationPoint {
    pub spot: f64,
    pub valuation_date: NaiveDate,
    pub volatility_shift: f64,
    pub theoretical_value: f64,
    pub pnl: f64,
    pub greeks: Greeks,
    pub legs: Vec<LegSimulationResult>,
    pub warnings: Vec<SimulationWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationResult {
    pub strategy_id: Option<String>,
    pub model: PricingModel,
    pub points: Vec<SimulationPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationWarning {
    AtOrAfterExpiration,
    VolatilityFloored,
}

pub trait StrategySimulator {
    fn simulate_strategy(
        &self,
        request: &SimulationRequest,
    ) -> Result<SimulationResult, SimulationError>;
}

#[derive(Debug, Default)]
pub struct BlackScholesSimulator;

impl StrategySimulator for BlackScholesSimulator {
    fn simulate_strategy(
        &self,
        request: &SimulationRequest,
    ) -> Result<SimulationResult, SimulationError> {
        validate_request(request)?;
        let mut points = Vec::with_capacity(
            request.grid.spots.len()
                * request.grid.valuation_dates.len()
                * request.grid.volatility_shifts.len(),
        );
        for &date in &request.grid.valuation_dates {
            for &shift in &request.grid.volatility_shifts {
                for &spot in &request.grid.spots {
                    points.push(simulate_point(request, spot, date, shift)?);
                }
            }
        }
        Ok(SimulationResult {
            strategy_id: request.strategy.id.clone(),
            model: PricingModel::BlackScholes,
            points,
        })
    }
}

fn simulate_point(
    request: &SimulationRequest,
    spot: f64,
    valuation_date: NaiveDate,
    volatility_shift: f64,
) -> Result<SimulationPoint, SimulationError> {
    let mut value = 0.0;
    let mut pnl = 0.0;
    let mut greeks = Greeks::default();
    let mut legs = Vec::with_capacity(request.strategy.legs.len());
    let mut warnings = Vec::new();
    for leg in &request.strategy.legs {
        let base_volatility = leg.entry_volatility.unwrap_or(request.market.volatility);
        let raw_volatility = base_volatility + volatility_shift;
        let volatility = raw_volatility.max(MIN_VOLATILITY);
        if raw_volatility < MIN_VOLATILITY
            && !warnings.contains(&SimulationWarning::VolatilityFloored)
        {
            warnings.push(SimulationWarning::VolatilityFloored);
        }
        let days = (leg.contract.expiration - valuation_date).num_days();
        let intrinsic = intrinsic_value(leg.contract.option_type, spot, leg.contract.strike);
        let priced = if days <= 0 {
            if !warnings.contains(&SimulationWarning::AtOrAfterExpiration) {
                warnings.push(SimulationWarning::AtOrAfterExpiration);
            }
            PriceWithGreeks {
                price: intrinsic,
                greeks: Greeks::default(),
            }
        } else {
            black_scholes(
                leg.contract.option_type,
                spot,
                leg.contract.strike,
                days as f64 / DAYS_PER_YEAR,
                request.market.risk_free_rate,
                request.market.dividend_yield,
                volatility,
            )?
        };
        let scale = leg.quantity as f64 * leg.multiplier as f64;
        let position_value = priced.price * scale;
        let leg_pnl = position_value - leg.entry_price * scale - leg.fees;
        let leg_greeks = priced.greeks.scaled(scale);
        value += position_value;
        pnl += leg_pnl;
        greeks.add_assign(leg_greeks);
        legs.push(LegSimulationResult {
            symbol: leg.contract.symbol.clone(),
            theoretical_price: priced.price,
            position_value,
            pnl: leg_pnl,
            intrinsic_value: intrinsic * scale,
            temporal_value: (priced.price - intrinsic).max(0.0) * scale,
            greeks: leg_greeks,
        });
    }
    Ok(SimulationPoint {
        spot,
        valuation_date,
        volatility_shift,
        theoretical_value: value,
        pnl,
        greeks,
        legs,
        warnings,
    })
}

#[derive(Debug, Clone, Copy)]
struct PriceWithGreeks {
    price: f64,
    greeks: Greeks,
}

fn black_scholes(
    option_type: OptionType,
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> Result<PriceWithGreeks, SimulationError> {
    if spot <= 0.0 || strike <= 0.0 || time <= 0.0 || volatility <= 0.0 {
        return Err(SimulationError::InvalidMarketInput);
    }
    let sqrt_time = time.sqrt();
    let d1 = ((spot / strike).ln()
        + (rate - dividend_yield + 0.5 * volatility * volatility) * time)
        / (volatility * sqrt_time);
    let d2 = d1 - volatility * sqrt_time;
    let discount_rate = (-rate * time).exp();
    let discount_dividend = (-dividend_yield * time).exp();
    let (price, delta, rho) = match option_type {
        OptionType::Call => (
            spot * discount_dividend * normal_cdf(d1) - strike * discount_rate * normal_cdf(d2),
            discount_dividend * normal_cdf(d1),
            strike * time * discount_rate * normal_cdf(d2) / 100.0,
        ),
        OptionType::Put => (
            strike * discount_rate * normal_cdf(-d2) - spot * discount_dividend * normal_cdf(-d1),
            discount_dividend * (normal_cdf(d1) - 1.0),
            -strike * time * discount_rate * normal_cdf(-d2) / 100.0,
        ),
    };
    let density = normal_pdf(d1);
    let gamma = discount_dividend * density / (spot * volatility * sqrt_time);
    let vega = spot * discount_dividend * density * sqrt_time / 100.0;
    let common_theta = -spot * discount_dividend * density * volatility / (2.0 * sqrt_time);
    let theta = match option_type {
        OptionType::Call => {
            common_theta - rate * strike * discount_rate * normal_cdf(d2)
                + dividend_yield * spot * discount_dividend * normal_cdf(d1)
        }
        OptionType::Put => {
            common_theta + rate * strike * discount_rate * normal_cdf(-d2)
                - dividend_yield * spot * discount_dividend * normal_cdf(-d1)
        }
    } / DAYS_PER_YEAR;
    Ok(PriceWithGreeks {
        price,
        greeks: Greeks {
            delta,
            gamma,
            vega,
            theta,
            rho,
        },
    })
}

/// Returns the Black-Scholes gamma used by strategy simulation.
pub fn black_scholes_gamma(
    spot: f64,
    strike: f64,
    time: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> Result<f64, SimulationError> {
    black_scholes(
        OptionType::Call,
        spot,
        strike,
        time,
        rate,
        dividend_yield,
        volatility,
    )
    .map(|priced| priced.greeks.gamma)
}

fn intrinsic_value(option_type: OptionType, spot: f64, strike: f64) -> f64 {
    match option_type {
        OptionType::Call => (spot - strike).max(0.0),
        OptionType::Put => (strike - spot).max(0.0),
    }
}

fn normal_pdf(value: f64) -> f64 {
    (-0.5 * value * value).exp() / (2.0 * PI).sqrt()
}

fn normal_cdf(value: f64) -> f64 {
    let absolute = value.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * absolute);
    let polynomial = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let positive = 1.0 - normal_pdf(absolute) * polynomial;
    if value >= 0.0 {
        positive
    } else {
        1.0 - positive
    }
}

fn validate_request(request: &SimulationRequest) -> Result<(), SimulationError> {
    if request.strategy.legs.is_empty()
        || request.grid.spots.is_empty()
        || request.grid.valuation_dates.is_empty()
        || request.grid.volatility_shifts.is_empty()
    {
        return Err(SimulationError::InvalidGrid);
    }
    if request.market.spot <= 0.0 || request.market.volatility <= 0.0 {
        return Err(SimulationError::InvalidMarketInput);
    }
    for leg in &request.strategy.legs {
        if leg.quantity == 0
            || leg.multiplier == 0
            || leg.entry_price < 0.0
            || leg.fees < 0.0
            || leg.contract.strike <= 0.0
        {
            return Err(SimulationError::InvalidLeg(leg.contract.symbol.clone()));
        }
        if leg.contract.exercise_style != ExerciseStyle::European
            || request.pricing.european_model != PricingModel::BlackScholes
        {
            return Err(SimulationError::UnsupportedModel);
        }
    }
    if request.grid.spots.iter().any(|spot| *spot <= 0.0) {
        return Err(SimulationError::InvalidGrid);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    InvalidGrid,
    InvalidMarketInput,
    InvalidLeg(String),
    UnsupportedModel,
}

impl fmt::Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGrid => write!(formatter, "a grelha de cenários é inválida"),
            Self::InvalidMarketInput => write!(formatter, "os dados de mercado são inválidos"),
            Self::InvalidLeg(symbol) => write!(formatter, "perna inválida: {symbol}"),
            Self::UnsupportedModel => write!(formatter, "modelo de pricing não suportado"),
        }
    }
}

impl Error for SimulationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn reproduces_canonical_black_scholes_prices_and_greeks() {
        let call = black_scholes(OptionType::Call, 100.0, 100.0, 1.0, 0.05, 0.0, 0.2).unwrap();
        let put = black_scholes(OptionType::Put, 100.0, 100.0, 1.0, 0.05, 0.0, 0.2).unwrap();

        assert!((call.price - 10.4506).abs() < 1.0e-3);
        assert!((put.price - 5.5735).abs() < 1.0e-3);
        assert!((call.greeks.delta - 0.6368).abs() < 1.0e-3);
        assert!((call.greeks.gamma - 0.01876).abs() < 1.0e-4);
        assert!((call.greeks.vega - 0.3752).abs() < 1.0e-3);
    }

    #[test]
    fn aggregates_a_multi_leg_strategy_across_the_scenario_grid() {
        let expiration = date(2026, 8, 16);
        let leg = |option_type: OptionType, symbol: &str| StrategyLeg {
            contract: OptionContract {
                symbol: symbol.to_string(),
                option_type,
                exercise_style: ExerciseStyle::European,
                strike: 100.0,
                expiration,
            },
            quantity: 1,
            multiplier: 100,
            entry_price: 5.0,
            entry_volatility: Some(0.25),
            fees: 1.0,
        };
        let request = SimulationRequest {
            strategy: Strategy {
                id: Some("straddle".to_string()),
                root: "TEST".to_string(),
                legs: vec![
                    leg(OptionType::Call, "TEST-C"),
                    leg(OptionType::Put, "TEST-P"),
                ],
            },
            market: MarketState {
                valuation_date: date(2026, 7, 17),
                spot: 100.0,
                risk_free_rate: 0.03,
                dividend_yield: 0.0,
                volatility: 0.25,
                snapshot_id: None,
            },
            grid: ScenarioGrid {
                spots: vec![80.0, 100.0, 120.0],
                valuation_dates: vec![date(2026, 7, 17), expiration],
                volatility_shifts: vec![0.0],
            },
            pricing: PricingConfig {
                european_model: PricingModel::BlackScholes,
                american_model: PricingModel::Binomial { steps: 200 },
            },
        };

        let result = BlackScholesSimulator.simulate_strategy(&request).unwrap();

        assert_eq!(result.points.len(), 6);
        let expiry_at_120 = result
            .points
            .iter()
            .find(|point| point.valuation_date == expiration && point.spot == 120.0)
            .unwrap();
        assert_eq!(expiry_at_120.theoretical_value, 2000.0);
        assert_eq!(expiry_at_120.pnl, 998.0);
        assert!(
            expiry_at_120
                .warnings
                .contains(&SimulationWarning::AtOrAfterExpiration)
        );
    }

    #[test]
    fn calendar_spread_keeps_long_leg_time_value_at_first_expiration() {
        let near_expiration = date(2026, 8, 16);
        let far_expiration = date(2026, 9, 16);
        let leg = |symbol: &str, expiration: NaiveDate, quantity: i32| StrategyLeg {
            contract: OptionContract {
                symbol: symbol.to_string(),
                option_type: OptionType::Call,
                exercise_style: ExerciseStyle::European,
                strike: 100.0,
                expiration,
            },
            quantity,
            multiplier: 100,
            entry_price: 5.0,
            entry_volatility: Some(0.25),
            fees: 0.0,
        };
        let request = SimulationRequest {
            strategy: Strategy {
                id: Some("calendar".to_string()),
                root: "TEST".to_string(),
                legs: vec![
                    leg("TEST-NEAR", near_expiration, -1),
                    leg("TEST-FAR", far_expiration, 1),
                ],
            },
            market: MarketState {
                valuation_date: date(2026, 7, 17),
                spot: 100.0,
                risk_free_rate: 0.03,
                dividend_yield: 0.0,
                volatility: 0.25,
                snapshot_id: None,
            },
            grid: ScenarioGrid {
                spots: vec![100.0],
                valuation_dates: vec![near_expiration],
                volatility_shifts: vec![0.0],
            },
            pricing: PricingConfig {
                european_model: PricingModel::BlackScholes,
                american_model: PricingModel::Binomial { steps: 200 },
            },
        };

        let result = BlackScholesSimulator.simulate_strategy(&request).unwrap();
        let point = &result.points[0];

        assert!(point.theoretical_value > 0.0);
        assert!(point.greeks.vega > 0.0);
        assert_eq!(point.legs[0].theoretical_price, 0.0);
        assert!(point.legs[1].temporal_value > 0.0);
    }

    #[test]
    fn builds_the_default_41_spot_grid() {
        let grid = ScenarioGrid::centered(
            100.0,
            0.2,
            41,
            vec![date(2026, 7, 17)],
            vec![-0.1, 0.0, 0.1],
        )
        .unwrap();

        assert_eq!(grid.spots.len(), 41);
        assert_eq!(grid.spots[0], 80.0);
        assert_eq!(grid.spots[40], 120.0);
    }

    #[test]
    fn includes_a_critical_spot_in_order() {
        let mut grid =
            ScenarioGrid::centered(101.0, 0.2, 5, vec![date(2026, 7, 17)], vec![0.0]).unwrap();

        grid.include_spot(100.0).unwrap();
        grid.include_spot(100.0).unwrap();

        assert_eq!(grid.spots.iter().filter(|spot| **spot == 100.0).count(), 1);
        assert!(grid.spots.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
