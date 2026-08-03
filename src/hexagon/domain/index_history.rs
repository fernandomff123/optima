use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexHistory {
    pub ticker: String,
    pub daily_prices: Vec<DailyIndexPrice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyIndexPrice {
    pub date: NaiveDate,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
}
