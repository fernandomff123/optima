use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnderlyingPriceObservation {
    pub value: f64,
    pub observed_at: Option<DateTime<Utc>>,
    pub currency: Option<String>,
    pub source: String,
    pub observed_at_raw: Option<String>,
    pub observed_at_timezone: Option<ProviderTimestampTimezone>,
}

impl UnderlyingPriceObservation {
    pub fn new(
        value: f64,
        observed_at: Option<DateTime<Utc>>,
        currency: Option<String>,
        source: impl Into<String>,
    ) -> Option<Self> {
        (value.is_finite() && value > 0.0).then(|| Self {
            value,
            observed_at,
            currency,
            source: source.into(),
            observed_at_raw: None,
            observed_at_timezone: None,
        })
    }

    pub fn with_provider_timestamp(
        mut self,
        raw: Option<String>,
        timezone: Option<ProviderTimestampTimezone>,
    ) -> Self {
        self.observed_at_raw = raw;
        self.observed_at_timezone = timezone;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderTimestampTimezone {
    VerifiedOffset,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderTimestamp {
    pub raw: String,
    pub timezone: ProviderTimestampTimezone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionContractSpecification {
    pub root: String,
    pub contract_multiplier: f64,
    pub currency: String,
    pub source_reference: String,
    pub catalog_reviewed_at: Option<NaiveDate>,
    pub effective_from: Option<NaiveDate>,
}

impl OptionContractSpecification {
    pub fn new(
        root: impl Into<String>,
        contract_multiplier: f64,
        currency: impl Into<String>,
        source_reference: impl Into<String>,
        catalog_reviewed_at: Option<NaiveDate>,
        effective_from: Option<NaiveDate>,
    ) -> Option<Self> {
        (contract_multiplier.is_finite() && contract_multiplier > 0.0).then(|| Self {
            root: root.into(),
            contract_multiplier,
            currency: currency.into(),
            source_reference: source_reference.into(),
            catalog_reviewed_at,
            effective_from,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionIngestionWarning {
    ProviderTimestampTimezoneUnverified,
    ContractSpecificationUnavailable { root: String },
}

pub const INVALID_OCC_SYMBOL_SAMPLE_LIMIT: usize = 20;
pub const INGESTION_WARNING_SAMPLE_LIMIT: usize = 50;
pub const DIAGNOSTIC_TEXT_LIMIT: usize = 128;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionIngestionDiagnostics {
    #[serde(default)]
    pub invalid_occ_symbol_count: u64,
    #[serde(default, alias = "invalid_occ_symbols")]
    pub invalid_occ_symbol_samples: Vec<String>,
    #[serde(default)]
    pub warning_count: u64,
    #[serde(default)]
    pub warnings: Vec<OptionIngestionWarning>,
}

impl OptionIngestionDiagnostics {
    pub fn record_invalid_occ_symbol(&mut self, symbol: String) {
        self.invalid_occ_symbol_count = self.invalid_occ_symbol_count.saturating_add(1);
        if self.invalid_occ_symbol_samples.len() < INVALID_OCC_SYMBOL_SAMPLE_LIMIT {
            self.invalid_occ_symbol_samples
                .push(bounded_diagnostic_text(symbol));
        }
    }

    pub fn record_warning(&mut self, warning: OptionIngestionWarning) {
        self.warning_count = self.warning_count.saturating_add(1);
        if self.warnings.len() < INGESTION_WARNING_SAMPLE_LIMIT {
            self.warnings.push(match warning {
                OptionIngestionWarning::ContractSpecificationUnavailable { root } => {
                    OptionIngestionWarning::ContractSpecificationUnavailable {
                        root: bounded_diagnostic_text(root),
                    }
                }
                warning => warning,
            });
        }
    }

    fn is_empty(&self) -> bool {
        self.invalid_occ_symbol_count == 0
            && self.invalid_occ_symbol_samples.is_empty()
            && self.warning_count == 0
            && self.warnings.is_empty()
    }
}

fn bounded_diagnostic_text(value: String) -> String {
    value.chars().take(DIAGNOSTIC_TEXT_LIMIT).collect()
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_specification: Option<OptionContractSpecification>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_price: Option<UnderlyingPriceObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collected_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_timestamp: Option<ProviderTimestamp>,
    #[serde(default, skip_serializing_if = "OptionIngestionDiagnostics::is_empty")]
    pub ingestion_diagnostics: OptionIngestionDiagnostics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_positive_numeric_inputs() {
        for value in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert!(UnderlyingPriceObservation::new(value, None, None, "source").is_none());
            assert!(
                OptionContractSpecification::new(
                    "SPX",
                    value,
                    "USD",
                    "source",
                    NaiveDate::from_ymd_opt(2026, 8, 20),
                    None,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn limits_invalid_occ_symbol_samples_while_preserving_the_total() {
        let mut diagnostics = OptionIngestionDiagnostics::default();
        for index in 0..100 {
            diagnostics.record_invalid_occ_symbol(format!("invalid-{index}-{}", "x".repeat(1_000)));
        }
        assert_eq!(diagnostics.invalid_occ_symbol_count, 100);
        assert_eq!(
            diagnostics.invalid_occ_symbol_samples.len(),
            INVALID_OCC_SYMBOL_SAMPLE_LIMIT
        );
        assert!(diagnostics.invalid_occ_symbol_samples[0].len() <= DIAGNOSTIC_TEXT_LIMIT);
    }

    #[test]
    fn limits_typed_warning_samples_and_root_length() {
        let mut diagnostics = OptionIngestionDiagnostics::default();
        for index in 0..100 {
            diagnostics.record_warning(OptionIngestionWarning::ContractSpecificationUnavailable {
                root: format!("{index}-{}", "x".repeat(1_000)),
            });
        }
        assert_eq!(diagnostics.warning_count, 100);
        assert_eq!(diagnostics.warnings.len(), INGESTION_WARNING_SAMPLE_LIMIT);
        let OptionIngestionWarning::ContractSpecificationUnavailable { root } =
            &diagnostics.warnings[0]
        else {
            panic!("expected root warning");
        };
        assert!(root.chars().count() <= DIAGNOSTIC_TEXT_LIMIT);
    }
}
