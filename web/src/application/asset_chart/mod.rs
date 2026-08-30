use std::rc::Rc;

use crate::{
    application::technical_indicators::TechnicalIndicatorsUseCase,
    domain::asset::{AssetCapability, AssetSymbol},
    ports::{
        asset_chart::{
            AssetChartFailure, AssetChartPort, AssetChartSnapshot, ChartCandleSnapshot,
            ChartScenario, GexLevelKind,
        },
        technical_indicators::{
            IndicatorPlacement, IndicatorRequest, MovingAverageKind, TechnicalCandle,
        },
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct ChartCandle {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartTone {
    Blue,
    Orange,
    Purple,
    Green,
    Red,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartLine {
    pub label: String,
    pub values: Vec<f64>,
    pub tone: ChartTone,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartIndicator {
    pub id: &'static str,
    pub label: &'static str,
    pub placement: IndicatorPlacement,
    pub lines: Vec<ChartLine>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GexLevel {
    pub label: String,
    pub value: f64,
    pub tone: ChartTone,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetChartReadModel {
    pub symbol: String,
    pub name: String,
    pub venue: String,
    pub price: String,
    pub absolute_change: String,
    pub percentage_change: String,
    pub change_positive: bool,
    pub market_status: String,
    pub capabilities: Vec<AssetCapability>,
    pub candles: Vec<ChartCandle>,
    pub indicators: Vec<ChartIndicator>,
    pub average_volume: f64,
    pub gex_levels: Vec<GexLevel>,
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
    indicators: TechnicalIndicatorsUseCase,
}

impl AssetChartUseCase {
    pub fn new(port: Rc<dyn AssetChartPort>, indicators: TechnicalIndicatorsUseCase) -> Self {
        Self { port, indicators }
    }

    pub fn execute(&self, ticker: &str, scenario: ChartScenario) -> AssetChartState {
        let symbol = AssetSymbol::new(ticker);
        if scenario == ChartScenario::Loading {
            return AssetChartState::Loading;
        }
        match self.port.load(&symbol, scenario) {
            Ok(Some(snapshot)) => self
                .to_read_model(snapshot)
                .map(AssetChartState::Ready)
                .unwrap_or_else(|| AssetChartState::RecoverableError {
                    symbol: symbol.as_str().to_owned(),
                }),
            Ok(None) => AssetChartState::Unavailable {
                symbol: symbol.as_str().to_owned(),
            },
            Err(AssetChartFailure::Recoverable) => AssetChartState::RecoverableError {
                symbol: symbol.as_str().to_owned(),
            },
        }
    }

    fn to_read_model(&self, snapshot: AssetChartSnapshot) -> Option<AssetChartReadModel> {
        let technical = snapshot
            .candles
            .iter()
            .map(technical_candle)
            .collect::<Vec<_>>();
        let mut indicators = Vec::with_capacity(6);
        for (id, label, request, tones) in indicator_requests() {
            let series = self.indicators.calculate(&technical, request).ok()?;
            let lines = series
                .lines
                .into_iter()
                .zip(tones)
                .map(|(line, tone)| ChartLine {
                    label: line.label.into(),
                    values: line.values,
                    tone,
                })
                .collect();
            indicators.push(ChartIndicator {
                id,
                label,
                placement: series.placement,
                lines,
            });
        }
        Some(AssetChartReadModel {
            symbol: snapshot.symbol.as_str().into(),
            name: snapshot.name.into(),
            venue: snapshot.venue.into(),
            price: snapshot.price.into(),
            absolute_change: snapshot.absolute_change.into(),
            percentage_change: snapshot.percentage_change.into(),
            change_positive: snapshot.change_positive,
            market_status: snapshot.market_status.into(),
            capabilities: snapshot.capabilities,
            candles: snapshot.candles.into_iter().map(chart_candle).collect(),
            indicators,
            average_volume: snapshot.average_volume,
            gex_levels: snapshot
                .gex_levels
                .into_iter()
                .map(|level| GexLevel {
                    label: level.label.into(),
                    value: level.value,
                    tone: match level.kind {
                        GexLevelKind::CallWall => ChartTone::Blue,
                        GexLevelKind::GammaFlip => ChartTone::Green,
                        GexLevelKind::PutWall => ChartTone::Red,
                    },
                })
                .collect(),
        })
    }
}

type IndicatorSpec = (&'static str, &'static str, IndicatorRequest, Vec<ChartTone>);

fn indicator_requests() -> Vec<IndicatorSpec> {
    use ChartTone::*;
    vec![
        ("ma-20", "MA (20)", moving_average(20), vec![Blue]),
        ("ma-50", "MA (50)", moving_average(50), vec![Orange]),
        ("ma-200", "MA (200)", moving_average(200), vec![Purple]),
        (
            "bollinger-bands",
            "Bollinger Bands (20, 2)",
            IndicatorRequest::BollingerBands {
                period: 20,
                sigma: 2.0,
            },
            vec![Blue, Blue, Blue],
        ),
        (
            "rsi",
            "RSI (14)",
            IndicatorRequest::RelativeStrengthIndex { period: 14 },
            vec![Purple],
        ),
        (
            "macd",
            "MACD (12, 26, close)",
            IndicatorRequest::MovingAverageConvergenceDivergence {
                fast_period: 12,
                slow_period: 26,
                signal_period: 9,
            },
            vec![Blue, Orange, Green],
        ),
    ]
}

fn moving_average(period: u8) -> IndicatorRequest {
    IndicatorRequest::MovingAverage {
        kind: MovingAverageKind::Simple,
        period,
    }
}

fn technical_candle(candle: &ChartCandleSnapshot) -> TechnicalCandle {
    TechnicalCandle {
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
    }
}

fn chart_candle(candle: ChartCandleSnapshot) -> ChartCandle {
    ChartCandle {
        timestamp: candle.timestamp,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::technical_indicators::{
        IndicatorLine, IndicatorSeries, TechnicalIndicatorFailure, TechnicalIndicatorPort,
    };

    struct EmptyChartPort;
    impl AssetChartPort for EmptyChartPort {
        fn load(
            &self,
            _: &AssetSymbol,
            _: ChartScenario,
        ) -> Result<Option<AssetChartSnapshot>, AssetChartFailure> {
            Ok(None)
        }
    }

    struct RecordingIndicators;
    impl TechnicalIndicatorPort for RecordingIndicators {
        fn calculate(
            &self,
            candles: &[TechnicalCandle],
            request: IndicatorRequest,
        ) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
            let lines = if matches!(
                request,
                IndicatorRequest::MovingAverageConvergenceDivergence { .. }
            ) {
                vec!["MACD", "Signal", "Histogram"]
            } else {
                vec!["Value"]
            };
            Ok(IndicatorSeries {
                placement: IndicatorPlacement::SeparatePanel,
                lines: lines
                    .into_iter()
                    .map(|label| IndicatorLine {
                        label,
                        values: vec![1.0; candles.len()],
                    })
                    .collect(),
            })
        }
    }

    #[test]
    fn loading_and_unavailable_states_do_not_require_a_backend() {
        let use_case = AssetChartUseCase::new(
            Rc::new(EmptyChartPort),
            TechnicalIndicatorsUseCase::new(Rc::new(RecordingIndicators)),
        );
        assert_eq!(
            use_case.execute("AAPL", ChartScenario::Loading),
            AssetChartState::Loading
        );
        assert_eq!(
            use_case.execute("AAPL", ChartScenario::Normal),
            AssetChartState::Unavailable {
                symbol: "AAPL".into()
            }
        );
    }
}
