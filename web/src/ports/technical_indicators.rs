#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TechnicalCandle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IndicatorRequest {
    BollingerBands {
        period: u8,
        sigma: f64,
    },
    RelativeStrengthIndex {
        period: u8,
    },
    MovingAverageConvergenceDivergence {
        fast_period: u8,
        slow_period: u8,
        signal_period: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndicatorPlacement {
    PriceOverlay,
    SeparatePanel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IndicatorLine {
    pub label: &'static str,
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IndicatorSeries {
    pub placement: IndicatorPlacement,
    pub lines: Vec<IndicatorLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TechnicalIndicatorFailure {
    EmptyInput,
    InvalidParameters,
    CalculationFailed,
}

pub trait TechnicalIndicatorPort {
    fn calculate(
        &self,
        candles: &[TechnicalCandle],
        request: IndicatorRequest,
    ) -> Result<IndicatorSeries, TechnicalIndicatorFailure>;
}
