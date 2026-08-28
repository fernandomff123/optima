use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_overview::{
        AssetOverviewFailure, AssetOverviewPort, AssetOverviewSnapshot, OverviewScenario,
    },
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayMetric {
    pub label: String,
    pub value: Option<String>,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayTable {
    pub title: String,
    pub headings: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PriceVolumeChart {
    pub times: Vec<String>,
    pub prices: Vec<f64>,
    pub volumes: Vec<f64>,
    pub price_unit: String,
    pub volume_unit: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetOverviewReadModel {
    pub symbol: String,
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
    pub metrics: Vec<DisplayMetric>,
    pub chart: PriceVolumeChart,
    pub key_statistics: Vec<DisplayMetric>,
    pub performance: DisplayTable,
    pub index_facts: Vec<DisplayMetric>,
    pub options_snapshot: Vec<DisplayMetric>,
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
    let metrics = snapshot.metrics.into_iter().map(metric).collect();
    let key_statistics = snapshot.key_statistics.into_iter().map(metric).collect();
    let index_facts = snapshot.index_facts.into_iter().map(metric).collect();
    let options_snapshot = snapshot.options_snapshot.into_iter().map(metric).collect();
    AssetOverviewReadModel {
        symbol: snapshot.symbol.as_str().to_owned(),
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
        metrics,
        chart: PriceVolumeChart {
            times: snapshot
                .chart_times
                .into_iter()
                .map(str::to_owned)
                .collect(),
            prices: snapshot.chart_prices,
            volumes: snapshot.chart_volumes,
            price_unit: snapshot.currency.into(),
            volume_unit: "contracts".into(),
            description:
                "Intraday price line with trading volume bars from 09:30 to 16:00 Eastern Time."
                    .into(),
        },
        key_statistics,
        performance: DisplayTable {
            title: snapshot.performance.title.into(),
            headings: snapshot
                .performance
                .headings
                .into_iter()
                .map(str::to_owned)
                .collect(),
            rows: snapshot
                .performance
                .rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|cell| cell.map(str::to_owned))
                        .collect()
                })
                .collect(),
        },
        index_facts,
        options_snapshot,
    }
}

fn metric(value: crate::ports::asset_overview::SnapshotMetric) -> DisplayMetric {
    DisplayMetric {
        label: value.label.into(),
        value: value.value.map(str::to_owned),
        unit: value.unit.map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::asset_overview::{SnapshotMetric, SnapshotTable};
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
                metrics: vec![SnapshotMetric {
                    label: "Missing",
                    value: None,
                    unit: Some("USD"),
                }],
                chart_times: vec!["00:00"],
                chart_prices: vec![1.0],
                chart_volumes: vec![0.0],
                key_statistics: vec![],
                performance: SnapshotTable {
                    title: "Performance",
                    headings: vec!["Period"],
                    rows: vec![vec![None]],
                },
                index_facts: vec![],
                options_snapshot: vec![],
            }))
        }
    }

    #[test]
    fn use_case_calls_port_and_preserves_route_ticker_datetime_and_none() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        let use_case = AssetOverviewUseCase::new(port.clone());
        let AssetOverviewState::Ready(model) = use_case.execute("ndx", OverviewScenario::Normal)
        else {
            panic!("ready expected")
        };
        assert_eq!(port.0.get(), 1);
        assert_eq!(model.symbol, "NDX");
        assert_eq!(model.datetime, "2026-08-28T00:00:00-04:00");
        assert_eq!(model.metrics[0].value, None);
        assert_eq!(model.metrics[0].unit.as_deref(), Some("USD"));
    }

    #[test]
    fn loading_does_not_call_port() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        let state =
            AssetOverviewUseCase::new(port.clone()).execute("SPX", OverviewScenario::Loading);
        assert_eq!(state, AssetOverviewState::Loading);
        assert_eq!(port.0.get(), 0);
    }
}
