use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptionType {
    Call,
    Put,
}

impl fmt::Display for OptionType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call => write!(formatter, "Call"),
            Self::Put => write!(formatter, "Put"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OccSymbol {
    pub root: String,
    pub expiration: NaiveDate,
    pub option_type: OptionType,
    pub strike: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ContratoOpcao {
    pub occ_symbol: String,
    pub option_type: OptionType,
    pub strike: f64,
    pub expiration: NaiveDate,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub spread: f64,
    pub volume: f64,
    pub open_interest: f64,
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
    pub theo: f64,
    pub implied_volatility: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OptionChain {
    pub root: String,
    pub contratos: Vec<ContratoOpcao>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Snapshot {
    pub ticker: String,
    pub timestamp_utc: DateTime<Utc>,
    pub contratos: Vec<ContratoOpcao>,
    pub chains: Vec<OptionChain>,
}
