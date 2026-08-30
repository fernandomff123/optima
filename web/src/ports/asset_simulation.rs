use crate::domain::asset::{AssetCapability, AssetSymbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimulationScenario {
    Normal,
    Loading,
    Unavailable,
    RecoverableError,
}

impl SimulationScenario {
    pub fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("loading") => Self::Loading,
            Some("unavailable") => Self::Unavailable,
            Some("recoverable-error") => Self::RecoverableError,
            _ => Self::Normal,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationLegSnapshot {
    pub quantity: i32,
    pub option_type: &'static str,
    pub strike: &'static str,
    pub expiration: &'static str,
    pub price: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PayoffPointSnapshot {
    pub underlying_price: f64,
    pub current_pnl: f64,
    pub expiration_pnl: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationMetricSnapshot {
    pub label: &'static str,
    pub value: &'static str,
    pub sentiment: MetricSentiment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricSentiment {
    Neutral,
    Positive,
    Negative,
    Special,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PnlHeatmapSnapshot {
    pub spot_prices: Vec<f64>,
    pub implied_volatilities: Vec<f64>,
    pub values: Vec<Vec<f64>>,
    pub selected_row: usize,
    pub selected_column: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GreekSnapshot {
    pub name: &'static str,
    pub value: &'static str,
    pub sensitivity: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioControlSnapshot {
    pub label: &'static str,
    pub current: &'static str,
    pub target: &'static str,
    pub minimum: &'static str,
    pub maximum: &'static str,
    pub position_percent: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetSimulationSnapshot {
    pub symbol: AssetSymbol,
    pub name: &'static str,
    pub venue: &'static str,
    pub price: &'static str,
    pub percentage_change: &'static str,
    pub change_positive: bool,
    pub capabilities: Vec<AssetCapability>,
    pub strategy_name: &'static str,
    pub legs: Vec<SimulationLegSnapshot>,
    pub payoff: Vec<PayoffPointSnapshot>,
    pub current_spot: f64,
    pub breakeven: f64,
    pub current_date: &'static str,
    pub expiration_date: &'static str,
    pub probability_low: &'static str,
    pub probability_high: &'static str,
    pub metrics: Vec<SimulationMetricSnapshot>,
    pub heatmap: PnlHeatmapSnapshot,
    pub greeks: Vec<GreekSnapshot>,
    pub preset: &'static str,
    pub controls: Vec<ScenarioControlSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetSimulationFailure {
    Recoverable,
}

pub trait AssetSimulationPort {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: SimulationScenario,
    ) -> Result<Option<AssetSimulationSnapshot>, AssetSimulationFailure>;
}
