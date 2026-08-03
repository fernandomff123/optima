use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Volatility {
    pub expiration: NaiveDate,
    pub time_to_expiration: f64,
    pub forward: f64,
    pub k0: f64,
    pub variance: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstantMaturityVolatility {
    pub target_days: f64,
    pub near_expiration: NaiveDate,
    pub next_expiration: NaiveDate,
    pub variance: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstantMaturityVolatilityPoint {
    pub date: NaiveDate,
    pub target_days: f64,
    pub volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermStructure {
    pub ticker: String,
    pub snapshot_timestamp: DateTime<Utc>,
    pub treasury_date: NaiveDate,
    pub points: Vec<TermStructurePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TermStructurePoint {
    pub days: f64,
    pub variance: f64,
    pub volatility: f64,
    pub source: TermStructureSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TermStructureSource {
    Interpolated {
        near_expiration: NaiveDate,
        near_rate: f64,
        next_expiration: NaiveDate,
        next_rate: f64,
    },
    Expiration {
        expiration: NaiveDate,
        interest_rate: f64,
    },
}
