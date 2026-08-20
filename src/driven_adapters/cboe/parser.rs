use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use chrono::{DateTime, Utc};

use super::client::{CboeOptionRowRaw, CboeResponse};
use crate::hexagon::domain::options::{
    ContratoOpcao, OccSymbol, OptionChain, OptionIngestionDiagnostics, OptionIngestionWarning,
    ProviderTimestamp, ProviderTimestampTimezone, Snapshot, UnderlyingPriceObservation,
};

#[derive(Debug)]
pub struct ParseError;

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Falha ao processar a string no padrão OCC")
    }
}

pub fn parse_occ_symbol(symbol: &str) -> Result<OccSymbol, ParseError> {
    OccSymbol::parse(symbol).map_err(|_| ParseError)
}

pub fn response_to_snapshot(
    ticker: &str,
    response: CboeResponse,
) -> Result<Snapshot, Box<dyn Error + Send + Sync>> {
    response_to_snapshot_collected_at(ticker, response, Utc::now())
}

pub fn response_to_snapshot_collected_at(
    ticker: &str,
    response: CboeResponse,
    collected_at: DateTime<Utc>,
) -> Result<Snapshot, Box<dyn Error + Send + Sync>> {
    let (provider_observed_at, timezone) = parse_cboe_timestamp(&response.timestamp);
    let provider_timestamp = ProviderTimestamp {
        raw: response.timestamp,
        timezone: timezone.clone(),
    };
    let mut diagnostics = OptionIngestionDiagnostics::default();
    if timezone == ProviderTimestampTimezone::Unverified {
        diagnostics.record_warning(OptionIngestionWarning::ProviderTimestampTimezoneUnverified);
    }
    let mut chains_by_root: HashMap<String, Vec<ContratoOpcao>> = HashMap::new();

    for option in response.data.options {
        let symbol = option.option.clone();
        match raw_option_to_domain(option) {
            Some((root, contrato)) => chains_by_root.entry(root).or_default().push(contrato),
            None => diagnostics.record_invalid_occ_symbol(symbol),
        }
    }

    let chains: Vec<OptionChain> = chains_by_root
        .into_iter()
        .map(|(root, contratos)| OptionChain { root, contratos })
        .collect();

    let contratos = chains
        .iter()
        .flat_map(|chain| chain.contratos.iter().cloned())
        .collect();

    Ok(Snapshot {
        ticker: ticker.to_uppercase(),
        // Retained for historical consumers. When the provider offset is not
        // evidenced this is collection time, not a fabricated market time.
        timestamp_utc: provider_observed_at.unwrap_or(collected_at),
        contratos,
        chains,
        underlying_price: response.data.current_price.and_then(|value| {
            let spot_timestamp_raw = response.data.last_trade_time;
            let (spot_as_of, spot_timezone) = spot_timestamp_raw
                .as_deref()
                .map(parse_cboe_timestamp)
                .unwrap_or((provider_observed_at, timezone.clone()));
            UnderlyingPriceObservation::new(value, spot_as_of, None, "cboe_delayed_quotes").map(
                |observation| {
                    observation.with_provider_timestamp(spot_timestamp_raw, Some(spot_timezone))
                },
            )
        }),
        collected_at: Some(collected_at),
        provider_timestamp: Some(provider_timestamp),
        ingestion_diagnostics: diagnostics,
    })
}

fn raw_option_to_domain(raw: CboeOptionRowRaw) -> Option<(String, ContratoOpcao)> {
    let occ = parse_occ_symbol(&raw.option).ok()?;
    let root = occ.root.clone();

    let contrato = ContratoOpcao {
        occ_symbol: raw.option,
        option_type: occ.option_type,
        strike: occ.strike,
        expiration: occ.expiration,
        bid: raw.bid,
        ask: raw.ask,
        mid: (raw.bid + raw.ask) / 2.0,
        spread: raw.ask - raw.bid,
        volume: raw.volume,
        open_interest: raw.open_interest,
        delta: raw.delta,
        gamma: raw.gamma,
        vega: raw.vega,
        theta: raw.theta,
        rho: raw.rho,
        theo: raw.theo,
        implied_volatility: (raw.iv.is_finite() && raw.iv > 0.0).then_some(raw.iv),
        contract_specification: None,
    };

    Some((root, contrato))
}

fn parse_cboe_timestamp(value: &str) -> (Option<DateTime<Utc>>, ProviderTimestampTimezone) {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return (
            Some(timestamp.with_timezone(&Utc)),
            ProviderTimestampTimezone::VerifiedOffset,
        );
    }
    (None, ProviderTimestampTimezone::Unverified)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::driven_adapters::cboe::client::{CboeData, CboeOptionRowRaw};

    fn row(symbol: &str) -> CboeOptionRowRaw {
        CboeOptionRowRaw {
            option: symbol.to_string(),
            bid: 1.0,
            ask: 1.2,
            volume: 10.0,
            open_interest: 20.0,
            delta: 0.5,
            gamma: 0.02,
            vega: 0.1,
            theta: -0.03,
            rho: 0.01,
            theo: 1.1,
            iv: 0.2,
        }
    }

    #[test]
    fn preserves_spot_and_verified_economic_timestamp() {
        let collected_at = Utc.with_ymd_and_hms(2026, 8, 20, 15, 1, 0).unwrap();
        let snapshot = response_to_snapshot_collected_at(
            "SPX",
            CboeResponse {
                timestamp: "2026-08-20T11:00:00-04:00".to_string(),
                data: CboeData {
                    options: vec![row("SPXW  260821C05000000")],
                    current_price: Some(6_420.5),
                    last_trade_time: Some("2026-08-20T11:00:00-04:00".to_string()),
                },
            },
            collected_at,
        )
        .unwrap();

        let spot = snapshot.underlying_price.unwrap();
        assert_eq!(spot.value, 6_420.5);
        assert_eq!(spot.observed_at, Some(snapshot.timestamp_utc));
        assert_eq!(snapshot.collected_at, Some(collected_at));
        assert_ne!(spot.observed_at, snapshot.collected_at);
    }

    #[test]
    fn does_not_promote_offsetless_timestamp_and_reports_invalid_occ() {
        let collected_at = Utc.with_ymd_and_hms(2026, 8, 20, 15, 1, 0).unwrap();
        let snapshot = response_to_snapshot_collected_at(
            "SPX",
            CboeResponse {
                timestamp: "2026-08-20 11:00:00".to_string(),
                data: CboeData {
                    options: vec![row("invalid"), row("SPXW  260821C05000000")],
                    current_price: Some(6_420.5),
                    last_trade_time: Some("2026-08-20T11:00:00".to_string()),
                },
            },
            collected_at,
        )
        .unwrap();

        assert_eq!(snapshot.timestamp_utc, collected_at);
        assert_eq!(snapshot.underlying_price.unwrap().observed_at, None);
        assert_eq!(
            snapshot.ingestion_diagnostics.invalid_occ_symbol_samples,
            ["invalid"]
        );
        assert_eq!(snapshot.ingestion_diagnostics.invalid_occ_symbol_count, 1);
        assert!(
            snapshot
                .ingestion_diagnostics
                .warnings
                .contains(&OptionIngestionWarning::ProviderTimestampTimezoneUnverified)
        );
    }

    #[test]
    fn invalid_spot_values_remain_absent() {
        for value in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            let response = CboeResponse {
                timestamp: "2026-08-20T15:00:00Z".to_string(),
                data: CboeData {
                    options: vec![row("SPXW  260821C05000000")],
                    current_price: Some(value),
                    last_trade_time: None,
                },
            };
            assert!(
                response_to_snapshot("SPX", response)
                    .unwrap()
                    .underlying_price
                    .is_none()
            );
        }
    }
}
