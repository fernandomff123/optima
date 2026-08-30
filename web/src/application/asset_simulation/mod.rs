use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_simulation::{
        AssetSimulationFailure, AssetSimulationPort, AssetSimulationSnapshot, GreekSnapshot,
        MetricSentiment, PayoffPointSnapshot, PnlHeatmapSnapshot, ScenarioControlSnapshot,
        SimulationLegSnapshot, SimulationScenario,
    },
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationLeg {
    pub quantity: i32,
    pub option_type: String,
    pub strike: String,
    pub expiration: String,
    pub price: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PayoffPoint {
    pub underlying_price: f64,
    pub current_pnl: f64,
    pub expiration_pnl: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationMetric {
    pub label: String,
    pub value: String,
    pub sentiment: MetricSentiment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PnlHeatmap {
    pub spot_prices: Vec<f64>,
    pub implied_volatilities: Vec<f64>,
    pub values: Vec<Vec<f64>>,
    pub selected_row: usize,
    pub selected_column: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Greek {
    pub name: String,
    pub value: String,
    pub sensitivity: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioControl {
    pub label: String,
    pub current: String,
    pub target: String,
    pub minimum: String,
    pub maximum: String,
    pub position_percent: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetSimulationReadModel {
    pub symbol: String,
    pub name: String,
    pub venue: String,
    pub price: String,
    pub percentage_change: String,
    pub change_positive: bool,
    pub capabilities: Vec<AssetCapability>,
    pub strategy_name: String,
    pub legs: Vec<SimulationLeg>,
    pub payoff: Vec<PayoffPoint>,
    pub current_spot: f64,
    pub breakeven: f64,
    pub current_date: String,
    pub expiration_date: String,
    pub probability_low: String,
    pub probability_high: String,
    pub metrics: Vec<SimulationMetric>,
    pub heatmap: PnlHeatmap,
    pub greeks: Vec<Greek>,
    pub preset: String,
    pub controls: Vec<ScenarioControl>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetSimulationState {
    Loading,
    Ready(AssetSimulationReadModel),
    Unavailable { symbol: String },
    RecoverableError { symbol: String },
}

#[derive(Clone)]
pub struct AssetSimulationUseCase {
    port: Rc<dyn AssetSimulationPort>,
}

impl AssetSimulationUseCase {
    pub fn new(port: Rc<dyn AssetSimulationPort>) -> Self {
        Self { port }
    }

    pub fn execute(&self, ticker: &str, scenario: SimulationScenario) -> AssetSimulationState {
        let symbol = AssetSymbol::new(ticker);
        if scenario == SimulationScenario::Loading {
            return AssetSimulationState::Loading;
        }
        match self.port.load(&symbol, scenario) {
            Ok(Some(snapshot)) => AssetSimulationState::Ready(to_read_model(snapshot)),
            Ok(None) => AssetSimulationState::Unavailable {
                symbol: symbol.as_str().to_owned(),
            },
            Err(AssetSimulationFailure::Recoverable) => AssetSimulationState::RecoverableError {
                symbol: symbol.as_str().to_owned(),
            },
        }
    }
}

fn to_read_model(snapshot: AssetSimulationSnapshot) -> AssetSimulationReadModel {
    AssetSimulationReadModel {
        symbol: snapshot.symbol.as_str().to_owned(),
        name: snapshot.name.into(),
        venue: snapshot.venue.into(),
        price: snapshot.price.into(),
        percentage_change: snapshot.percentage_change.into(),
        change_positive: snapshot.change_positive,
        capabilities: snapshot.capabilities,
        strategy_name: snapshot.strategy_name.into(),
        legs: snapshot.legs.into_iter().map(leg).collect(),
        payoff: snapshot.payoff.into_iter().map(payoff_point).collect(),
        current_spot: snapshot.current_spot,
        breakeven: snapshot.breakeven,
        current_date: snapshot.current_date.into(),
        expiration_date: snapshot.expiration_date.into(),
        probability_low: snapshot.probability_low.into(),
        probability_high: snapshot.probability_high.into(),
        metrics: snapshot
            .metrics
            .into_iter()
            .map(|metric| SimulationMetric {
                label: metric.label.into(),
                value: metric.value.into(),
                sentiment: metric.sentiment,
            })
            .collect(),
        heatmap: heatmap(snapshot.heatmap),
        greeks: snapshot.greeks.into_iter().map(greek).collect(),
        preset: snapshot.preset.into(),
        controls: snapshot.controls.into_iter().map(control).collect(),
    }
}

fn leg(value: SimulationLegSnapshot) -> SimulationLeg {
    SimulationLeg {
        quantity: value.quantity,
        option_type: value.option_type.into(),
        strike: value.strike.into(),
        expiration: value.expiration.into(),
        price: value.price.into(),
    }
}

fn payoff_point(value: PayoffPointSnapshot) -> PayoffPoint {
    PayoffPoint {
        underlying_price: value.underlying_price,
        current_pnl: value.current_pnl,
        expiration_pnl: value.expiration_pnl,
    }
}

fn heatmap(value: PnlHeatmapSnapshot) -> PnlHeatmap {
    PnlHeatmap {
        spot_prices: value.spot_prices,
        implied_volatilities: value.implied_volatilities,
        values: value.values,
        selected_row: value.selected_row,
        selected_column: value.selected_column,
    }
}

fn greek(value: GreekSnapshot) -> Greek {
    Greek {
        name: value.name.into(),
        value: value.value.into(),
        sensitivity: value.sensitivity.into(),
    }
}

fn control(value: ScenarioControlSnapshot) -> ScenarioControl {
    ScenarioControl {
        label: value.label.into(),
        current: value.current.into(),
        target: value.target.into(),
        minimum: value.minimum.into(),
        maximum: value.maximum.into(),
        position_percent: value.position_percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::asset_simulation::{
        AssetSimulationSnapshot, PnlHeatmapSnapshot, SimulationMetricSnapshot,
    };
    use std::cell::Cell;

    struct RecordingPort(Cell<usize>);

    impl AssetSimulationPort for RecordingPort {
        fn load(
            &self,
            symbol: &AssetSymbol,
            _: SimulationScenario,
        ) -> Result<Option<AssetSimulationSnapshot>, AssetSimulationFailure> {
            self.0.set(self.0.get() + 1);
            Ok(Some(AssetSimulationSnapshot {
                symbol: symbol.clone(),
                name: "Apple Inc.",
                venue: "NASDAQ",
                price: "$191.13",
                percentage_change: "+1.24%",
                change_positive: true,
                capabilities: vec![AssetCapability::Simulation],
                strategy_name: "Long Call Spread",
                legs: vec![],
                payoff: vec![PayoffPointSnapshot {
                    underlying_price: 191.13,
                    current_pnl: 225.0,
                    expiration_pnl: 0.0,
                }],
                current_spot: 191.13,
                breakeven: 192.8,
                current_date: "May 10, 2024",
                expiration_date: "May 17, 2024",
                probability_low: "172.20",
                probability_high: "210.20",
                metrics: vec![SimulationMetricSnapshot {
                    label: "POP",
                    value: "56%",
                    sentiment: MetricSentiment::Neutral,
                }],
                heatmap: PnlHeatmapSnapshot {
                    spot_prices: vec![191.13],
                    implied_volatilities: vec![23.8],
                    values: vec![vec![225.0]],
                    selected_row: 0,
                    selected_column: 0,
                },
                greeks: vec![],
                preset: "Base",
                controls: vec![],
            }))
        }
    }

    #[test]
    fn use_case_reads_the_provider_neutral_simulation_snapshot() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        let AssetSimulationState::Ready(model) =
            AssetSimulationUseCase::new(port.clone()).execute("aapl", SimulationScenario::Normal)
        else {
            panic!("ready simulation expected")
        };
        assert_eq!(port.0.get(), 1);
        assert_eq!(model.symbol, "AAPL");
        assert_eq!(model.payoff[0].current_pnl, 225.0);
        assert_eq!(model.heatmap.values, vec![vec![225.0]]);
    }

    #[test]
    fn loading_does_not_call_the_port() {
        let port = Rc::new(RecordingPort(Cell::new(0)));
        assert_eq!(
            AssetSimulationUseCase::new(port.clone()).execute("AAPL", SimulationScenario::Loading),
            AssetSimulationState::Loading
        );
        assert_eq!(port.0.get(), 0);
    }
}
