use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketHistory {
    pub ticker: String,
    pub currency: Option<String>,
    pub exchange_timezone: Option<String>,
    pub daily_quotes: Vec<DailyQuote>,
    pub dividends: Vec<Dividend>,
    pub splits: Vec<StockSplit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyQuote {
    pub timestamp: DateTime<Utc>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub adjusted_close: Option<f64>,
    pub volume: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dividend {
    pub timestamp: DateTime<Utc>,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockSplit {
    pub timestamp: DateTime<Utc>,
    pub numerator: f64,
    pub denominator: f64,
    pub ratio: String,
}
