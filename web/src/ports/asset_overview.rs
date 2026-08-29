use crate::domain::asset::{AssetCapability, AssetKind, AssetSymbol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverviewScenario {
    Normal,
    Loading,
    Stale,
    Partial,
    Unavailable,
    RecoverableError,
    TerminalError,
}

impl OverviewScenario {
    pub fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("loading") => Self::Loading,
            Some("stale") => Self::Stale,
            Some("partial") => Self::Partial,
            Some("unavailable") => Self::Unavailable,
            Some("recoverable-error") => Self::RecoverableError,
            Some("terminal-error") => Self::TerminalError,
            _ => Self::Normal,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotMetric {
    pub label: &'static str,
    pub value: Option<&'static str>,
    pub suffix: Option<&'static str>,
    pub unit: Option<&'static str>,
    pub tone: SnapshotTone,
    pub numeric: bool,
    pub starts_group: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotTone {
    Positive,
    Negative,
    Neutral,
    Special,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotTable {
    pub title: &'static str,
    pub headings: Vec<&'static str>,
    pub rows: Vec<Vec<Option<&'static str>>>,
    pub tones: Vec<Vec<SnapshotTone>>,
    pub suffixes: Vec<Vec<Option<&'static str>>>,
    pub units: Vec<Vec<Option<&'static str>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotRange {
    pub label: &'static str,
    pub minimum: &'static str,
    pub maximum: &'static str,
    pub position: f64,
    pub insert_after: usize,
    pub accessible_value: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotNewsItem {
    pub headline: &'static str,
    pub source: &'static str,
    pub age: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetOverviewSnapshot {
    pub symbol: AssetSymbol,
    pub kind: AssetKind,
    pub name: &'static str,
    pub venue: &'static str,
    pub price: &'static str,
    pub absolute_change: &'static str,
    pub percentage_change: &'static str,
    pub change_positive: bool,
    pub currency: &'static str,
    pub market_status: &'static str,
    pub observed_at: &'static str,
    pub datetime: &'static str,
    pub freshness: &'static str,
    pub is_stale: bool,
    pub is_mock: bool,
    pub capabilities: Vec<AssetCapability>,
    pub day_range: Option<SnapshotMetric>,
    pub chart_times: Vec<String>,
    pub chart_prices: Vec<f64>,
    pub chart_volumes: Vec<f64>,
    pub chart_session_end: &'static str,
    pub chart_last_price: &'static str,
    pub chart_last_volume: &'static str,
    pub key_statistics: Vec<SnapshotMetric>,
    pub year_range: Option<SnapshotRange>,
    pub performance: SnapshotTable,
    pub earnings: Option<Vec<SnapshotMetric>>,
    pub index_facts: Option<Vec<SnapshotMetric>>,
    pub options_snapshot: Vec<SnapshotMetric>,
    pub latest_news: Option<Vec<SnapshotNewsItem>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetOverviewFailure {
    Recoverable,
    Terminal,
}

pub trait AssetOverviewPort {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: OverviewScenario,
    ) -> Result<Option<AssetOverviewSnapshot>, AssetOverviewFailure>;
}
