use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_volatility::{
        AssetVolatilityFailure, AssetVolatilityPort, AssetVolatilitySnapshot,
        VolatilityGridSnapshot, VolatilityHistoryPointSnapshot, VolatilityHistorySummarySnapshot,
        VolatilityMetricSnapshot, VolatilityScenario, VolatilitySmileSnapshot,
        VolatilityTermPointSnapshot,
    },
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilityGrid {
    pub moneyness: Vec<f64>,
    pub days_to_expiry: Vec<u16>,
    pub implied_volatility_percent: Vec<Vec<f64>>,
    pub selected_moneyness_index: usize,
    pub selected_expiry_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilitySmile {
    pub label: String,
    pub days_to_expiry: u16,
    pub implied_volatility_percent: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolatilityTermPoint {
    pub days_to_expiry: u16,
    pub implied_volatility_percent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilityMetric {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilityHistoryPoint {
    pub label: String,
    pub atm_iv_30d_percent: f64,
    pub realized_volatility_20d_percent: f64,
    pub realized_volatility_60d_percent: f64,
    pub earnings: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilityHistorySummary {
    pub atm_iv_30d: String,
    pub rv20: String,
    pub rv60: String,
    pub iv_rv_spread_30d: String,
    pub percentile: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetVolatilityReadModel {
    pub symbol: String,
    pub name: String,
    pub venue: String,
    pub price: String,
    pub percentage_change: String,
    pub change_positive: bool,
    pub capabilities: Vec<AssetCapability>,
    pub metric: String,
    pub option_type: String,
    pub expiration_filter: String,
    pub normalization: String,
    pub as_of: String,
    pub grid: VolatilityGrid,
    pub smiles: Vec<VolatilitySmile>,
    pub term_structure: Vec<VolatilityTermPoint>,
    pub history: Vec<VolatilityHistoryPoint>,
    pub history_summary: VolatilityHistorySummary,
    pub snapshot_metrics: Vec<VolatilityMetric>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetVolatilityState {
    Loading,
    Ready(AssetVolatilityReadModel),
    Unavailable { symbol: String },
    RecoverableError { symbol: String },
}

#[derive(Clone)]
pub struct AssetVolatilityUseCase {
    port: Rc<dyn AssetVolatilityPort>,
}

impl AssetVolatilityUseCase {
    pub fn new(port: Rc<dyn AssetVolatilityPort>) -> Self {
        Self { port }
    }

    pub fn execute(&self, ticker: &str, scenario: VolatilityScenario) -> AssetVolatilityState {
        let symbol = AssetSymbol::new(ticker);
        if scenario == VolatilityScenario::Loading {
            return AssetVolatilityState::Loading;
        }
        match self.port.load(&symbol, scenario) {
            Ok(Some(snapshot)) => AssetVolatilityState::Ready(to_read_model(snapshot)),
            Ok(None) => AssetVolatilityState::Unavailable {
                symbol: symbol.as_str().to_owned(),
            },
            Err(AssetVolatilityFailure::Recoverable) => AssetVolatilityState::RecoverableError {
                symbol: symbol.as_str().to_owned(),
            },
        }
    }
}

fn to_read_model(snapshot: AssetVolatilitySnapshot) -> AssetVolatilityReadModel {
    AssetVolatilityReadModel {
        symbol: snapshot.symbol.as_str().to_owned(),
        name: snapshot.name.into(),
        venue: snapshot.venue.into(),
        price: snapshot.price.into(),
        percentage_change: snapshot.percentage_change.into(),
        change_positive: snapshot.change_positive,
        capabilities: snapshot.capabilities,
        metric: snapshot.metric.into(),
        option_type: snapshot.option_type.into(),
        expiration_filter: snapshot.expiration_filter.into(),
        normalization: snapshot.normalization.into(),
        as_of: snapshot.as_of.into(),
        grid: grid(snapshot.grid),
        smiles: snapshot.smiles.into_iter().map(smile).collect(),
        term_structure: snapshot
            .term_structure
            .into_iter()
            .map(term_point)
            .collect(),
        history: snapshot.history.into_iter().map(history_point).collect(),
        history_summary: history_summary(snapshot.history_summary),
        snapshot_metrics: snapshot.snapshot_metrics.into_iter().map(metric).collect(),
    }
}

fn grid(value: VolatilityGridSnapshot) -> VolatilityGrid {
    VolatilityGrid {
        moneyness: value.moneyness,
        days_to_expiry: value.days_to_expiry,
        implied_volatility_percent: value.implied_volatility_percent,
        selected_moneyness_index: value.selected_moneyness_index,
        selected_expiry_index: value.selected_expiry_index,
    }
}

fn smile(value: VolatilitySmileSnapshot) -> VolatilitySmile {
    VolatilitySmile {
        label: value.label.into(),
        days_to_expiry: value.days_to_expiry,
        implied_volatility_percent: value.implied_volatility_percent,
    }
}

fn term_point(value: VolatilityTermPointSnapshot) -> VolatilityTermPoint {
    VolatilityTermPoint {
        days_to_expiry: value.days_to_expiry,
        implied_volatility_percent: value.implied_volatility_percent,
    }
}

fn metric(value: VolatilityMetricSnapshot) -> VolatilityMetric {
    VolatilityMetric {
        label: value.label.into(),
        value: value.value.into(),
    }
}

fn history_point(value: VolatilityHistoryPointSnapshot) -> VolatilityHistoryPoint {
    VolatilityHistoryPoint {
        label: value.label.into(),
        atm_iv_30d_percent: value.atm_iv_30d_percent,
        realized_volatility_20d_percent: value.realized_volatility_20d_percent,
        realized_volatility_60d_percent: value.realized_volatility_60d_percent,
        earnings: value.earnings,
    }
}

fn history_summary(value: VolatilityHistorySummarySnapshot) -> VolatilityHistorySummary {
    VolatilityHistorySummary {
        atm_iv_30d: value.atm_iv_30d.into(),
        rv20: value.rv20.into(),
        rv60: value.rv60.into(),
        iv_rv_spread_30d: value.iv_rv_spread_30d.into(),
        percentile: value.percentile.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::asset_volatility::{AssetVolatilityFailure, AssetVolatilitySnapshot};
    use std::{cell::Cell, rc::Rc};

    struct StubPort {
        calls: Rc<Cell<u8>>,
        result: Result<Option<AssetVolatilitySnapshot>, AssetVolatilityFailure>,
    }

    impl AssetVolatilityPort for StubPort {
        fn load(
            &self,
            _symbol: &AssetSymbol,
            _scenario: VolatilityScenario,
        ) -> Result<Option<AssetVolatilitySnapshot>, AssetVolatilityFailure> {
            self.calls.set(self.calls.get() + 1);
            self.result.clone()
        }
    }

    #[test]
    fn loading_does_not_call_the_port() {
        let calls = Rc::new(Cell::new(0));
        let use_case = AssetVolatilityUseCase::new(Rc::new(StubPort {
            calls: calls.clone(),
            result: Ok(None),
        }));

        assert_eq!(
            use_case.execute("aapl", VolatilityScenario::Loading),
            AssetVolatilityState::Loading
        );
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn use_case_keeps_the_surface_provider_neutral() {
        let snapshot = AssetVolatilitySnapshot {
            symbol: AssetSymbol::new("aapl"),
            name: "Apple Inc.",
            venue: "NASDAQ",
            price: "$191.13",
            percentage_change: "+1.24%",
            change_positive: true,
            capabilities: vec![AssetCapability::Volatility],
            metric: "Implied Volatility",
            option_type: "Calls + Puts",
            expiration_filter: "All",
            normalization: "Moneyness",
            as_of: "Illustrative fixture",
            grid: VolatilityGridSnapshot {
                moneyness: vec![1.0],
                days_to_expiry: vec![30],
                implied_volatility_percent: vec![vec![24.4]],
                selected_moneyness_index: 0,
                selected_expiry_index: 0,
            },
            smiles: vec![],
            term_structure: vec![],
            history: vec![],
            history_summary: VolatilityHistorySummarySnapshot {
                atm_iv_30d: "24%",
                rv20: "19%",
                rv60: "20%",
                iv_rv_spread_30d: "+5 vols",
                percentile: "60%",
            },
            snapshot_metrics: vec![],
        };
        let use_case = AssetVolatilityUseCase::new(Rc::new(StubPort {
            calls: Rc::new(Cell::new(0)),
            result: Ok(Some(snapshot)),
        }));

        let AssetVolatilityState::Ready(model) =
            use_case.execute("aapl", VolatilityScenario::Normal)
        else {
            panic!("expected ready state");
        };
        assert_eq!(model.grid.implied_volatility_percent, vec![vec![24.4]]);
        assert_eq!(model.normalization, "Moneyness");
    }
}
