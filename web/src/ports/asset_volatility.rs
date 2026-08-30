use crate::domain::asset::{AssetCapability, AssetSymbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VolatilityScenario {
    Normal,
    Loading,
    Unavailable,
    RecoverableError,
}

impl VolatilityScenario {
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
pub struct VolatilityGridSnapshot {
    pub moneyness: Vec<f64>,
    pub days_to_expiry: Vec<u16>,
    pub implied_volatility_percent: Vec<Vec<f64>>,
    pub selected_moneyness_index: usize,
    pub selected_expiry_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilitySmileSnapshot {
    pub label: &'static str,
    pub days_to_expiry: u16,
    pub implied_volatility_percent: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolatilityTermPointSnapshot {
    pub days_to_expiry: u16,
    pub implied_volatility_percent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilityMetricSnapshot {
    pub label: &'static str,
    pub value: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilityHistoryPointSnapshot {
    pub label: &'static str,
    pub atm_iv_30d_percent: f64,
    pub realized_volatility_20d_percent: f64,
    pub realized_volatility_60d_percent: f64,
    pub earnings: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetVolatilitySnapshot {
    pub symbol: AssetSymbol,
    pub name: &'static str,
    pub venue: &'static str,
    pub price: &'static str,
    pub percentage_change: &'static str,
    pub change_positive: bool,
    pub capabilities: Vec<AssetCapability>,
    pub metric: &'static str,
    pub option_type: &'static str,
    pub expiration_filter: &'static str,
    pub normalization: &'static str,
    pub as_of: &'static str,
    pub grid: VolatilityGridSnapshot,
    pub smiles: Vec<VolatilitySmileSnapshot>,
    pub term_structure: Vec<VolatilityTermPointSnapshot>,
    pub history: Vec<VolatilityHistoryPointSnapshot>,
    pub snapshot_metrics: Vec<VolatilityMetricSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetVolatilityFailure {
    Recoverable,
}

pub trait AssetVolatilityPort {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: VolatilityScenario,
    ) -> Result<Option<AssetVolatilitySnapshot>, AssetVolatilityFailure>;
}
