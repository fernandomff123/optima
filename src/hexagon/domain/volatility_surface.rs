use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::hexagon::domain::options::{OptionType, Snapshot};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurface {
    pub ticker: String,
    pub snapshot_time: DateTime<Utc>,
    pub reference_price: f64,
    pub points: Vec<VolatilitySurfacePoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurfacePoint {
    pub expiration: NaiveDate,
    pub days_to_expiration: i64,
    pub strike: f64,
    pub moneyness: f64,
    pub option_type: OptionType,
    pub implied_volatility: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySkew {
    pub ticker: String,
    pub expiration: NaiveDate,
    pub points: Vec<VolatilitySurfacePoint>,
}

impl VolatilitySurface {
    pub fn from_snapshot(snapshot: &Snapshot, reference_price: f64) -> Option<Self> {
        if !reference_price.is_finite() || reference_price <= 0.0 {
            return None;
        }
        let session_date = snapshot.timestamp_utc.date_naive();
        let mut points = snapshot
            .contratos
            .iter()
            .filter_map(|contract| {
                let implied_volatility = contract.implied_volatility?;
                let days_to_expiration = contract
                    .expiration
                    .signed_duration_since(session_date)
                    .num_days();
                let is_otm = match contract.option_type {
                    OptionType::Put => contract.strike < reference_price,
                    OptionType::Call => contract.strike >= reference_price,
                };
                if days_to_expiration <= 0
                    || !is_otm
                    || contract.bid <= 0.0
                    || contract.ask < contract.bid
                    || contract.vega <= 0.0
                    || !implied_volatility.is_finite()
                    || implied_volatility <= 0.0
                {
                    return None;
                }
                Some(VolatilitySurfacePoint {
                    expiration: contract.expiration,
                    days_to_expiration,
                    strike: contract.strike,
                    moneyness: contract.strike / reference_price,
                    option_type: contract.option_type,
                    implied_volatility,
                })
            })
            .collect::<Vec<_>>();
        points.sort_by(|left, right| {
            left.expiration
                .cmp(&right.expiration)
                .then_with(|| left.strike.total_cmp(&right.strike))
        });
        (!points.is_empty()).then(|| Self {
            ticker: snapshot.ticker.clone(),
            snapshot_time: snapshot.timestamp_utc,
            reference_price,
            points,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::domain::options::{ContratoOpcao, OptionChain};
    use chrono::TimeZone;

    fn contract(option_type: OptionType, strike: f64, iv: Option<f64>) -> ContratoOpcao {
        ContratoOpcao {
            occ_symbol: format!("TEST-{strike}"),
            option_type,
            strike,
            expiration: NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
            bid: 1.0,
            ask: 1.2,
            mid: 1.1,
            spread: 0.2,
            volume: 10.0,
            open_interest: Some(100.0),
            delta: 0.4,
            gamma: Some(0.02),
            vega: 0.1,
            theta: -0.03,
            rho: 0.01,
            theo: 1.1,
            implied_volatility: iv,
            contract_specification: None,
        }
    }

    #[test]
    fn builds_the_surface_from_valid_otm_contract_iv() {
        let contracts = vec![
            contract(OptionType::Put, 95.0, Some(0.28)),
            contract(OptionType::Call, 105.0, Some(0.24)),
            contract(OptionType::Call, 95.0, Some(0.27)),
            contract(OptionType::Put, 90.0, None),
        ];
        let snapshot = Snapshot {
            ticker: "TEST".to_string(),
            timestamp_utc: Utc.with_ymd_and_hms(2026, 7, 15, 21, 0, 0).unwrap(),
            contratos: contracts.clone(),
            chains: vec![OptionChain {
                root: "TEST".to_string(),
                contratos: contracts,
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: Default::default(),
        };

        let surface = VolatilitySurface::from_snapshot(&snapshot, 100.0).unwrap();

        assert_eq!(surface.points.len(), 2);
        assert_eq!(surface.points[0].moneyness, 0.95);
        assert_eq!(surface.points[0].implied_volatility, 0.28);
        assert_eq!(surface.points[1].moneyness, 1.05);
    }
}
