use crate::{
    domain::asset::{AssetCapability, AssetKind, AssetSymbol},
    ports::asset_overview::{
        AssetOverviewFailure, AssetOverviewPort, AssetOverviewSnapshot, OverviewScenario,
        SnapshotMetric, SnapshotNewsItem, SnapshotTable, SnapshotTone,
    },
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayMetric {
    pub label: String,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub tone: ValueTone,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueTone {
    Positive,
    Negative,
    Neutral,
    Special,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayTable {
    pub title: String,
    pub headings: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub tones: Vec<Vec<ValueTone>>,
    pub units: Vec<Vec<Option<String>>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DisplayRange {
    pub label: String,
    pub minimum: String,
    pub maximum: String,
    pub position: f64,
    pub insert_after: usize,
    pub accessible_value: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NewsItem {
    pub headline: String,
    pub source: String,
    pub age: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PriceVolumeChart {
    pub times: Vec<String>,
    pub prices: Vec<f64>,
    pub volumes: Vec<f64>,
    pub session_end: String,
    pub last_price: String,
    pub last_volume: String,
    pub price_unit: String,
    pub volume_unit: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetOverviewReadModel {
    pub symbol: String,
    pub kind: AssetKind,
    pub name: String,
    pub venue: String,
    pub price: String,
    pub absolute_change: String,
    pub percentage_change: String,
    pub change_positive: bool,
    pub currency: String,
    pub market_status: String,
    pub observed_at: String,
    pub datetime: String,
    pub freshness: String,
    pub is_stale: bool,
    pub is_mock: bool,
    pub capabilities: Vec<AssetCapability>,
    pub day_range: Option<DisplayMetric>,
    pub chart: PriceVolumeChart,
    pub key_statistics: Vec<DisplayMetric>,
    pub year_range: Option<DisplayRange>,
    pub performance: DisplayTable,
    pub earnings: Option<Vec<DisplayMetric>>,
    pub index_facts: Option<Vec<DisplayMetric>>,
    pub options_snapshot: Vec<DisplayMetric>,
    pub latest_news: Option<Vec<NewsItem>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetOverviewState {
    Loading,
    Ready(AssetOverviewReadModel),
    Partial(AssetOverviewReadModel),
    Unavailable { symbol: String },
    RecoverableError { symbol: String },
    TerminalError { symbol: String },
}

#[derive(Clone)]
pub struct AssetOverviewUseCase {
    port: Rc<dyn AssetOverviewPort>,
}
impl AssetOverviewUseCase {
    pub fn new(port: Rc<dyn AssetOverviewPort>) -> Self {
        Self { port }
    }
    pub fn execute(&self, ticker: &str, scenario: OverviewScenario) -> AssetOverviewState {
        let symbol = AssetSymbol::new(ticker);
        if scenario == OverviewScenario::Loading {
            return AssetOverviewState::Loading;
        }
        match self.port.load(&symbol, scenario) {
            Ok(Some(snapshot)) => {
                let model = to_read_model(snapshot);
                if scenario == OverviewScenario::Partial {
                    AssetOverviewState::Partial(model)
                } else {
                    AssetOverviewState::Ready(model)
                }
            }
            Ok(None) => AssetOverviewState::Unavailable {
                symbol: symbol.as_str().to_owned(),
            },
            Err(AssetOverviewFailure::Recoverable) => AssetOverviewState::RecoverableError {
                symbol: symbol.as_str().to_owned(),
            },
            Err(AssetOverviewFailure::Terminal) => AssetOverviewState::TerminalError {
                symbol: symbol.as_str().to_owned(),
            },
        }
    }
}

fn to_read_model(snapshot: AssetOverviewSnapshot) -> AssetOverviewReadModel {
    let chart = PriceVolumeChart {
        times: snapshot.chart_times, prices: snapshot.chart_prices, volumes: snapshot.chart_volumes,
        session_end: snapshot.chart_session_end.into(), last_price: snapshot.chart_last_price.into(),
        last_volume: snapshot.chart_last_volume.into(), price_unit: snapshot.currency.into(), volume_unit: "shares".into(),
        description: "Intraday price and volume observations through 13:00 Eastern Time; the axis continues to the 16:00 session close.".into(),
    };
    AssetOverviewReadModel {
        symbol: snapshot.symbol.as_str().to_owned(),
        kind: snapshot.kind,
        name: snapshot.name.into(),
        venue: snapshot.venue.into(),
        price: snapshot.price.into(),
        absolute_change: snapshot.absolute_change.into(),
        percentage_change: snapshot.percentage_change.into(),
        change_positive: snapshot.change_positive,
        currency: snapshot.currency.into(),
        market_status: snapshot.market_status.into(),
        observed_at: snapshot.observed_at.into(),
        datetime: snapshot.datetime.into(),
        freshness: snapshot.freshness.into(),
        is_stale: snapshot.is_stale,
        is_mock: snapshot.is_mock,
        capabilities: snapshot.capabilities,
        day_range: snapshot.day_range.map(metric),
        chart,
        key_statistics: metrics(snapshot.key_statistics),
        year_range: snapshot.year_range.map(|range| DisplayRange {
            label: range.label.into(),
            minimum: range.minimum.into(),
            maximum: range.maximum.into(),
            position: range.position,
            insert_after: range.insert_after,
            accessible_value: range.accessible_value.into(),
        }),
        performance: table(snapshot.performance),
        earnings: snapshot.earnings.map(metrics),
        index_facts: snapshot.index_facts.map(metrics),
        options_snapshot: metrics(snapshot.options_snapshot),
        latest_news: snapshot
            .latest_news
            .map(|items| items.into_iter().map(news).collect()),
    }
}
fn metric(value: SnapshotMetric) -> DisplayMetric {
    DisplayMetric {
        label: value.label.into(),
        value: value.value.map(str::to_owned),
        unit: value.unit.map(str::to_owned),
        tone: tone(value.tone),
    }
}
fn metrics(values: Vec<SnapshotMetric>) -> Vec<DisplayMetric> {
    values.into_iter().map(metric).collect()
}
fn table(value: SnapshotTable) -> DisplayTable {
    DisplayTable {
        title: value.title.into(),
        headings: value.headings.into_iter().map(str::to_owned).collect(),
        rows: value
            .rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| cell.map(str::to_owned))
                    .collect()
            })
            .collect(),
        tones: value
            .tones
            .into_iter()
            .map(|row| row.into_iter().map(tone).collect())
            .collect(),
        units: value
            .units
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|unit| unit.map(str::to_owned))
                    .collect()
            })
            .collect(),
    }
}
fn tone(value: SnapshotTone) -> ValueTone {
    match value {
        SnapshotTone::Positive => ValueTone::Positive,
        SnapshotTone::Negative => ValueTone::Negative,
        SnapshotTone::Neutral => ValueTone::Neutral,
        SnapshotTone::Special => ValueTone::Special,
    }
}
fn news(value: SnapshotNewsItem) -> NewsItem {
    NewsItem {
        headline: value.headline.into(),
        source: value.source.into(),
        age: value.age.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::asset_overview::{SnapshotRange, SnapshotTable};
    use std::cell::Cell;
    struct RecordingPort(Cell<usize>);
    impl AssetOverviewPort for RecordingPort {
        fn load(
            &self,
            symbol: &AssetSymbol,
            _: OverviewScenario,
        ) -> Result<Option<AssetOverviewSnapshot>, AssetOverviewFailure> {
            self.0.set(self.0.get() + 1);
            Ok(Some(AssetOverviewSnapshot {
                symbol: symbol.clone(),
                kind: AssetKind::Index,
                name: "Index",
                venue: "INDEX",
                price: "1.00",
                absolute_change: "+0.00",
                percentage_change: "+0.00%",
                change_positive: true,
                currency: "USD",
                market_status: "CLOSED",
                observed_at: "00:00 ET",
                datetime: "2026-08-28T00:00:00-04:00",
                freshness: "Fixed",
                is_stale: false,
                is_mock: true,
                capabilities: vec![AssetCapability::Overview],
                day_range: None,
                chart_times: vec!["00:00".into()],
                chart_prices: vec![1.0],
                chart_volumes: vec![1.0],
                chart_session_end: "16:00",
                chart_last_price: "1.00",
                chart_last_volume: "1",
                key_statistics: vec![],
                year_range: Some(SnapshotRange {
                    label: "52W Range",
                    minimum: "0",
                    maximum: "2",
                    position: 0.5,
                    insert_after: 0,
                    accessible_value: "1 of 0 to 2",
                }),
                performance: SnapshotTable {
                    title: "Performance",
                    headings: vec!["Period"],
                    rows: vec![vec![None]],
                    tones: vec![vec![SnapshotTone::Neutral]],
                    units: vec![vec![None]],
                },
                earnings: None,
                index_facts: Some(vec![]),
                options_snapshot: vec![],
                latest_news: None,
            }))
        }
    }
    #[test]
    fn use_case_calls_port_and_preserves_provider_neutral_presentation_values() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        let AssetOverviewState::Ready(model) =
            AssetOverviewUseCase::new(port.clone()).execute("ndx", OverviewScenario::Normal)
        else {
            panic!("ready expected")
        };
        assert_eq!(port.0.get(), 1);
        assert_eq!(model.symbol, "NDX");
        assert_eq!(model.datetime, "2026-08-28T00:00:00-04:00");
        assert_eq!(model.year_range.unwrap().position, 0.5);
        assert_eq!(model.chart.session_end, "16:00");
    }
    #[test]
    fn loading_does_not_call_port() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        assert_eq!(
            AssetOverviewUseCase::new(port.clone()).execute("SPX", OverviewScenario::Loading),
            AssetOverviewState::Loading
        );
        assert_eq!(port.0.get(), 0);
    }
    #[test]
    fn provider_neutral_tones_are_preserved_for_the_ui() {
        assert_eq!(tone(SnapshotTone::Positive), ValueTone::Positive);
        assert_eq!(tone(SnapshotTone::Negative), ValueTone::Negative);
        assert_eq!(tone(SnapshotTone::Neutral), ValueTone::Neutral);
        assert_eq!(tone(SnapshotTone::Special), ValueTone::Special);
    }
}
