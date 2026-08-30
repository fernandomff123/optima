use crate::domain::asset::{AssetCapability, AssetSymbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartScenario {
    Normal,
    Loading,
    Unavailable,
    RecoverableError,
}

impl ChartScenario {
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
pub struct ChartCandleSnapshot {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GexLevelKind {
    CallWall,
    GammaFlip,
    PutWall,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GexLevelSnapshot {
    pub kind: GexLevelKind,
    pub label: &'static str,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetChartSnapshot {
    pub symbol: AssetSymbol,
    pub name: &'static str,
    pub venue: &'static str,
    pub price: &'static str,
    pub absolute_change: &'static str,
    pub percentage_change: &'static str,
    pub change_positive: bool,
    pub market_status: &'static str,
    pub capabilities: Vec<AssetCapability>,
    pub candles: Vec<ChartCandleSnapshot>,
    pub average_volume: f64,
    pub gex_levels: Vec<GexLevelSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetChartFailure {
    Recoverable,
}

pub trait AssetChartPort {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: ChartScenario,
    ) -> Result<Option<AssetChartSnapshot>, AssetChartFailure>;
}
