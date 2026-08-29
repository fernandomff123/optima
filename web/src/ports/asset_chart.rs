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
pub struct CandleSnapshot {
    pub date: &'static str,
    /// Prices are USD per share, ordered as open, close, low, high.
    pub ohlc: [f64; 4],
    /// Trading volume in shares.
    pub volume: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetChartSnapshot {
    pub symbol: AssetSymbol,
    pub name: &'static str,
    pub venue: &'static str,
    pub price: &'static str,
    pub absolute_change: &'static str,
    pub percentage_change: &'static str,
    pub market_status: &'static str,
    pub capabilities: Vec<AssetCapability>,
    pub candles: Vec<CandleSnapshot>,
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
