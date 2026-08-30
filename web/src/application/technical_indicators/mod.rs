use std::rc::Rc;

use crate::ports::technical_indicators::{
    IndicatorRequest, IndicatorSeries, TechnicalCandle, TechnicalIndicatorFailure,
    TechnicalIndicatorPort,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndicatorCategory {
    Overlay,
    Momentum,
    Trend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndicatorDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub category: IndicatorCategory,
}

pub const INITIAL_INDICATOR_CATALOG: [IndicatorDefinition; 3] = [
    IndicatorDefinition {
        id: "bollinger-bands",
        label: "Bollinger Bands",
        category: IndicatorCategory::Overlay,
    },
    IndicatorDefinition {
        id: "rsi",
        label: "Relative Strength Index",
        category: IndicatorCategory::Momentum,
    },
    IndicatorDefinition {
        id: "macd",
        label: "MACD",
        category: IndicatorCategory::Trend,
    },
];

#[derive(Clone)]
pub struct TechnicalIndicatorsUseCase {
    port: Rc<dyn TechnicalIndicatorPort>,
}

impl TechnicalIndicatorsUseCase {
    pub fn new(port: Rc<dyn TechnicalIndicatorPort>) -> Self {
        Self { port }
    }

    pub fn calculate(
        &self,
        candles: &[TechnicalCandle],
        request: IndicatorRequest,
    ) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
        self.port.calculate(candles, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::technical_indicators::{
        IndicatorLine, IndicatorPlacement, TechnicalIndicatorPort,
    };

    struct RecordingEngine;

    impl TechnicalIndicatorPort for RecordingEngine {
        fn calculate(
            &self,
            candles: &[TechnicalCandle],
            _: IndicatorRequest,
        ) -> Result<IndicatorSeries, TechnicalIndicatorFailure> {
            Ok(IndicatorSeries {
                placement: IndicatorPlacement::SeparatePanel,
                lines: vec![IndicatorLine {
                    label: "RSI",
                    values: vec![candles.len() as f64],
                }],
            })
        }
    }

    #[test]
    fn use_case_depends_on_the_provider_neutral_indicator_port() {
        let candles = [TechnicalCandle {
            open: 190.0,
            high: 192.0,
            low: 189.0,
            close: 191.0,
            volume: 50_000_000.0,
        }];
        let result = TechnicalIndicatorsUseCase::new(Rc::new(RecordingEngine))
            .calculate(
                &candles,
                IndicatorRequest::RelativeStrengthIndex { period: 14 },
            )
            .expect("indicator calculation should succeed");

        assert_eq!(result.lines[0].values, vec![1.0]);
        assert_eq!(INITIAL_INDICATOR_CATALOG.len(), 3);
    }
}
