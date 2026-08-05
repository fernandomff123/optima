use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use chrono::{DateTime, NaiveDateTime, Utc};

use super::client::{CboeOptionRowRaw, CboeResponse};
use crate::hexagon::domain::options::{ContratoOpcao, OccSymbol, OptionChain, Snapshot};

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
    let timestamp_utc = parse_cboe_timestamp(&response.timestamp)?;
    let mut chains_by_root: HashMap<String, Vec<ContratoOpcao>> = HashMap::new();

    for option in response.data.options {
        if let Some((root, contrato)) = raw_option_to_domain(option) {
            chains_by_root.entry(root).or_default().push(contrato);
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
        timestamp_utc,
        contratos,
        chains,
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
    };

    Some((root, contrato))
}

fn parse_cboe_timestamp(value: &str) -> Result<DateTime<Utc>, Box<dyn Error + Send + Sync>> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp.with_timezone(&Utc));
    }

    let timestamp = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")?;
    Ok(timestamp.and_utc())
}
