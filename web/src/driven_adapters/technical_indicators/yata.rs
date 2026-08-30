use crate::ports::technical_indicators::{
    IndicatorLine, IndicatorPlacement, IndicatorRequest, IndicatorSeries, TechnicalCandle,
    TechnicalIndicatorFailure, TechnicalIndicatorPort,
};
use yata::{
    core::Source,
    helpers::MA,
    indicators::{BollingerBands, MACD, RelativeStrengthIndex},
    prelude::{Candle, IndicatorConfig},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct YataTechnicalIndicatorAdapter;

impl TechnicalIndicatorPort for YataTechnicalIndicatorAdapter {
    fn calculate(
        &self,
        candles: &[TechnicalCandle],
        request: IndicatorRequest,
    ) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
        if candles.is_empty() {
            return Err(TechnicalIndicatorFailure::EmptyInput);
        }

        let yata_candles = candles
            .iter()
            .map(|candle| Candle {
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume,
            })
            .collect::<Vec<_>>();

        match request {
            IndicatorRequest::BollingerBands { period, sigma } => {
                bollinger_bands(&yata_candles, period, sigma)
            }
            IndicatorRequest::RelativeStrengthIndex { period } => {
                relative_strength_index(&yata_candles, period)
            }
            IndicatorRequest::MovingAverageConvergenceDivergence {
                fast_period,
                slow_period,
                signal_period,
            } => macd(&yata_candles, fast_period, slow_period, signal_period),
        }
    }
}

fn bollinger_bands(
    candles: &[Candle],
    period: u8,
    sigma: f64,
) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
    let output = BollingerBands {
        avg_size: period,
        sigma,
        source: Source::Close,
    }
    .over(candles)
    .map_err(map_yata_error)?;

    Ok(IndicatorSeries {
        placement: IndicatorPlacement::PriceOverlay,
        lines: vec![
            line("Upper", &output, 0, 1.0),
            line("Middle", &output, 1, 1.0),
            line("Lower", &output, 2, 1.0),
        ],
    })
}

fn relative_strength_index(
    candles: &[Candle],
    period: u8,
) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
    let output = RelativeStrengthIndex {
        ma: MA::EMA(period),
        zone: 0.3,
        source: Source::Close,
    }
    .over(candles)
    .map_err(map_yata_error)?;

    Ok(IndicatorSeries {
        placement: IndicatorPlacement::SeparatePanel,
        lines: vec![line("RSI", &output, 0, 100.0)],
    })
}

fn macd(
    candles: &[Candle],
    fast_period: u8,
    slow_period: u8,
    signal_period: u8,
) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
    let output = MACD {
        ma1: MA::EMA(fast_period),
        ma2: MA::EMA(slow_period),
        signal: MA::EMA(signal_period),
        source: Source::Close,
    }
    .over(candles)
    .map_err(map_yata_error)?;
    let macd = values(&output, 0, 1.0);
    let signal = values(&output, 1, 1.0);
    let histogram = macd
        .iter()
        .zip(&signal)
        .map(|(macd, signal)| macd - signal)
        .collect();

    Ok(IndicatorSeries {
        placement: IndicatorPlacement::SeparatePanel,
        lines: vec![
            IndicatorLine {
                label: "MACD",
                values: macd,
            },
            IndicatorLine {
                label: "Signal",
                values: signal,
            },
            IndicatorLine {
                label: "Histogram",
                values: histogram,
            },
        ],
    })
}

fn line(
    label: &'static str,
    output: &[yata::core::IndicatorResult],
    index: usize,
    scale: f64,
) -> IndicatorLine {
    IndicatorLine {
        label,
        values: values(output, index, scale),
    }
}

fn values(output: &[yata::core::IndicatorResult], index: usize, scale: f64) -> Vec<f64> {
    output
        .iter()
        .map(|result| result.value(index) * scale)
        .collect()
}

fn map_yata_error(_: yata::core::Error) -> TechnicalIndicatorFailure {
    TechnicalIndicatorFailure::InvalidParameters
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles() -> Vec<TechnicalCandle> {
        (0..80)
            .map(|index| {
                let close = 180.0 + index as f64 * 0.18 + (index as f64 / 4.0).sin() * 2.2;
                TechnicalCandle {
                    open: close - 0.4,
                    high: close + 1.1,
                    low: close - 1.2,
                    close,
                    volume: 42_000_000.0 + index as f64 * 125_000.0,
                }
            })
            .collect()
    }

    #[test]
    fn yata_calculates_overlay_and_panel_indicators_with_aligned_output() {
        let engine = YataTechnicalIndicatorAdapter;
        let candles = candles();

        let bollinger = engine
            .calculate(
                &candles,
                IndicatorRequest::BollingerBands {
                    period: 20,
                    sigma: 2.0,
                },
            )
            .unwrap();
        let rsi = engine
            .calculate(
                &candles,
                IndicatorRequest::RelativeStrengthIndex { period: 14 },
            )
            .unwrap();
        let macd = engine
            .calculate(
                &candles,
                IndicatorRequest::MovingAverageConvergenceDivergence {
                    fast_period: 12,
                    slow_period: 26,
                    signal_period: 9,
                },
            )
            .unwrap();

        assert_eq!(bollinger.placement, IndicatorPlacement::PriceOverlay);
        assert_eq!(bollinger.lines.len(), 3);
        assert_eq!(rsi.lines[0].values.len(), candles.len());
        assert!(
            rsi.lines[0]
                .values
                .iter()
                .all(|value| (0.0..=100.0).contains(value))
        );
        assert_eq!(macd.lines.len(), 3);
        assert!(
            macd.lines
                .iter()
                .all(|line| line.values.len() == candles.len())
        );
    }

    #[test]
    fn yata_rejects_empty_input_and_invalid_parameters() {
        let engine = YataTechnicalIndicatorAdapter;
        assert_eq!(
            engine.calculate(&[], IndicatorRequest::RelativeStrengthIndex { period: 14 }),
            Err(TechnicalIndicatorFailure::EmptyInput)
        );
        assert_eq!(
            engine.calculate(
                &candles(),
                IndicatorRequest::BollingerBands {
                    period: 1,
                    sigma: 2.0,
                }
            ),
            Err(TechnicalIndicatorFailure::InvalidParameters)
        );
    }
}
