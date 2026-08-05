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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OccSymbolError;

impl std::error::Error for OccSymbolError {}

impl fmt::Display for OccSymbolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid OCC option symbol")
    }
}

impl OccSymbol {
    /// Interprets the industry-standard OCC symbol without provider knowledge.
    pub fn parse(symbol: &str) -> Result<Self, OccSymbolError> {
        let len = symbol.len();
        if len < 16 || !symbol.is_char_boundary(len - 15) {
            return Err(OccSymbolError);
        }

        let strike_raw = &symbol[len - 8..];
        let option_type_raw = &symbol[len - 9..len - 8];
        let expiration_raw = &symbol[len - 15..len - 9];
        let option_type = match option_type_raw {
            "C" => OptionType::Call,
            "P" => OptionType::Put,
            _ => return Err(OccSymbolError),
        };

        Ok(Self {
            root: symbol[..len - 15].trim().to_string(),
            expiration: NaiveDate::parse_from_str(expiration_raw, "%y%m%d")
                .map_err(|_| OccSymbolError)?,
            option_type,
            strike: strike_raw
                .parse::<f64>()
                .map(|value| value / 1000.0)
                .map_err(|_| OccSymbolError)?,
        })
    }
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
