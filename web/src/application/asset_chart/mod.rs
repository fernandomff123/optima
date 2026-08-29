use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_chart::{AssetChartFailure, AssetChartPort, AssetChartSnapshot, ChartScenario},
};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct Candle {
    pub date: String,
    pub open: f64,
    pub close: f64,
    pub low: f64,
    pub high: f64,
    pub volume: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetChartReadModel {
    pub symbol: String,
    pub name: String,
    pub venue: String,
    pub price: String,
    pub absolute_change: String,
    pub percentage_change: String,
    pub market_status: String,
    pub capabilities: Vec<AssetCapability>,
    pub candles: Vec<Candle>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetChartState {
    Loading,
    Ready(AssetChartReadModel),
    Unavailable { symbol: String },
    RecoverableError { symbol: String },
}

#[derive(Clone)]
pub struct AssetChartUseCase {
    port: Rc<dyn AssetChartPort>,
}
impl AssetChartUseCase {
    pub fn new(port: Rc<dyn AssetChartPort>) -> Self {
        Self { port }
    }
    pub fn execute(&self, ticker: &str, scenario: ChartScenario) -> AssetChartState {
        let symbol = AssetSymbol::new(ticker);
        if scenario == ChartScenario::Loading {
            return AssetChartState::Loading;
        }
        match self.port.load(&symbol, scenario) {
            Ok(Some(value)) => AssetChartState::Ready(map(value)),
            Ok(None) => AssetChartState::Unavailable {
                symbol: symbol.as_str().into(),
            },
            Err(AssetChartFailure::Recoverable) => AssetChartState::RecoverableError {
                symbol: symbol.as_str().into(),
            },
        }
    }
}

fn map(value: AssetChartSnapshot) -> AssetChartReadModel {
    AssetChartReadModel {
        symbol: value.symbol.as_str().into(),
        name: value.name.into(),
        venue: value.venue.into(),
        price: value.price.into(),
        absolute_change: value.absolute_change.into(),
        percentage_change: value.percentage_change.into(),
        market_status: value.market_status.into(),
        capabilities: value.capabilities,
        candles: value
            .candles
            .into_iter()
            .map(|point| Candle {
                date: point.date.into(),
                open: point.ohlc[0],
                close: point.ohlc[1],
                low: point.ohlc[2],
                high: point.ohlc[3],
                volume: point.volume,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::asset_chart::CandleSnapshot;
    use std::cell::Cell;
    struct Port(Cell<usize>);
    impl AssetChartPort for Port {
        fn load(
            &self,
            symbol: &AssetSymbol,
            _: ChartScenario,
        ) -> Result<Option<AssetChartSnapshot>, AssetChartFailure> {
            self.0.set(self.0.get() + 1);
            Ok(Some(AssetChartSnapshot {
                symbol: symbol.clone(),
                name: "Apple Inc.",
                venue: "NASDAQ",
                price: "191.13",
                absolute_change: "+2.35",
                percentage_change: "+1.24%",
                market_status: "MARKET OPEN",
                capabilities: vec![AssetCapability::Chart],
                candles: vec![CandleSnapshot {
                    date: "May 01",
                    ohlc: [188.0, 191.0, 187.0, 192.0],
                    volume: 42_000_000,
                }],
            }))
        }
    }
    #[test]
    fn maps_provider_neutral_candles_in_documented_order() {
        let port = Rc::new(Port(Cell::new(0)));
        let AssetChartState::Ready(model) =
            AssetChartUseCase::new(port.clone()).execute("aapl", ChartScenario::Normal)
        else {
            panic!("ready expected")
        };
        assert_eq!(port.0.get(), 1);
        assert_eq!(
            (
                model.candles[0].open,
                model.candles[0].close,
                model.candles[0].low,
                model.candles[0].high
            ),
            (188.0, 191.0, 187.0, 192.0)
        );
    }
    #[test]
    fn loading_does_not_touch_port() {
        let port = Rc::new(Port(Cell::new(0)));
        assert_eq!(
            AssetChartUseCase::new(port.clone()).execute("AAPL", ChartScenario::Loading),
            AssetChartState::Loading
        );
        assert_eq!(port.0.get(), 0);
    }
}
