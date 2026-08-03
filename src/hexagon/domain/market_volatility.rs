use chrono::{DateTime, NaiveDate, Utc};

/// One observed volatility index value and its session-over-session change.
#[derive(Debug, Clone, PartialEq)]
pub struct VolatilityIndexValue {
    pub ticker: String,
    pub date: NaiveDate,
    pub close: f64,
    pub daily_change_percent: Option<f64>,
}

/// A constant-maturity volatility calculated from an option term structure.
#[derive(Debug, Clone, PartialEq)]
pub struct CalculatedVolatility {
    pub ticker: String,
    pub snapshot_timestamp: DateTime<Utc>,
    pub volatility_percent: f64,
    pub difference_from_vix: f64,
}

/// Domain result of the market-volatility overview conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketVolatilityOverview {
    pub as_of: NaiveDate,
    pub vix: VolatilityIndexValue,
    pub spx_30_day: Option<CalculatedVolatility>,
    pub vvix: Option<VolatilityIndexValue>,
    pub term_structure: Vec<VolatilityIndexValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImpliedVolatilityPoint {
    pub date: NaiveDate,
    pub volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImpliedVolatilityOverview {
    pub ticker: String,
    pub reference_ticker: Option<String>,
    pub points: Vec<ImpliedVolatilityPoint>,
}
