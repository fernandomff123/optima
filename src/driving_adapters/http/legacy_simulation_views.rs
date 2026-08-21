//! Compatibility mapping for the pre-hexagonal simulation HTTP contract.

use api_models::{
    SimulationCatalogOverview, SimulationContractOverview, SimulationCurveOverview,
    SimulationGreeksOverview, SimulationLegOverview, SimulationOverview, SimulationPointOverview,
};

use crate::hexagon::domain::options::{OptionType, Snapshot};
use crate::hexagon::domain::simulation::{
    SimulationScenario, SimulationStrategyKind, SimulationTradeSide,
};

/// Translates the hexagon's scenario result into the existing HTTP contract.
pub fn scenario(value: SimulationScenario) -> SimulationOverview {
    SimulationOverview {
        ticker: value.ticker,
        strategy_kind: match value.strategy_kind {
            SimulationStrategyKind::Straddle => api_models::SimulationStrategyKind::Straddle,
            SimulationStrategyKind::BullCallSpread => {
                api_models::SimulationStrategyKind::BullCallSpread
            }
            SimulationStrategyKind::Custom => api_models::SimulationStrategyKind::Custom,
        },
        strategy_label: value.strategy_label,
        valuation_date: value.valuation_date,
        expiration: value.expiration,
        strike: value.strike,
        upper_strike: value.upper_strike,
        spot: value.spot,
        model: value.model,
        break_even_points: value.break_even_points,
        legs: value
            .legs
            .into_iter()
            .map(|leg| SimulationLegOverview {
                occ_symbol: leg.occ_symbol,
                option_type: match leg.option_type {
                    OptionType::Call => "Call",
                    OptionType::Put => "Put",
                }
                .to_string(),
                strike: leg.strike,
                expiration: leg.expiration,
                side: match leg.side {
                    SimulationTradeSide::Buy => api_models::SimulationTradeSide::Buy,
                    SimulationTradeSide::Sell => api_models::SimulationTradeSide::Sell,
                },
                quantity: leg.quantity,
                entry_price: leg.entry_price,
                base_volatility: leg.base_volatility,
            })
            .collect(),
        curves: value
            .curves
            .into_iter()
            .map(|curve| SimulationCurveOverview {
                label: curve.label,
                valuation_date: curve.valuation_date,
                volatility_shift: curve.volatility_shift,
                volatility_limited: curve.volatility_limited,
                points: curve
                    .points
                    .into_iter()
                    .map(|point| SimulationPointOverview {
                        spot: point.spot,
                        pnl: point.pnl,
                        greeks: SimulationGreeksOverview {
                            delta: point.greeks.delta,
                            gamma: point.greeks.gamma,
                            theta: point.greeks.theta,
                            vega: point.greeks.vega,
                            rho: point.greeks.rho,
                        },
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn catalog(ticker: &str, snapshot: &Snapshot, spot: f64) -> SimulationCatalogOverview {
    let valuation_date = snapshot.timestamp_utc.date_naive();
    let expirations = snapshot
        .contratos
        .iter()
        .map(|contract| contract.expiration)
        .filter(|expiration| {
            let days = (*expiration - valuation_date).num_days();
            (1..=365).contains(&days)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut contracts = snapshot
        .contratos
        .iter()
        .filter(|contract| expirations.contains(&contract.expiration))
        .filter(|contract| contract.bid >= 0.0 && contract.ask >= contract.bid)
        .map(|contract| SimulationContractOverview {
            occ_symbol: contract.occ_symbol.clone(),
            option_type: match contract.option_type {
                OptionType::Call => "Call",
                OptionType::Put => "Put",
            }
            .to_string(),
            strike: contract.strike,
            expiration: contract.expiration,
            bid: contract.bid,
            ask: contract.ask,
            mid: contract.mid,
            implied_volatility: contract.implied_volatility,
            delta: contract.delta,
            gamma: contract.gamma,
            theta: contract.theta,
            vega: contract.vega,
            rho: contract.rho,
            volume: contract.volume,
            open_interest: contract.open_interest,
        })
        .collect::<Vec<_>>();
    contracts.sort_by(|left, right| {
        left.expiration
            .cmp(&right.expiration)
            .then_with(|| left.strike.total_cmp(&right.strike))
            .then_with(|| left.option_type.cmp(&right.option_type))
    });
    SimulationCatalogOverview {
        ticker: ticker.to_string(),
        snapshot_time: snapshot.timestamp_utc,
        spot,
        expirations,
        contracts,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::hexagon::domain::options::{ContratoOpcao, OptionChain};

    fn contract(symbol: &str, gamma: Option<f64>, open_interest: Option<f64>) -> ContratoOpcao {
        ContratoOpcao {
            occ_symbol: symbol.to_string(),
            option_type: OptionType::Call,
            strike: 100.0,
            expiration: NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            bid: 1.0,
            ask: 1.2,
            mid: 1.1,
            spread: 0.2,
            volume: 1.0,
            open_interest,
            delta: 0.5,
            gamma,
            vega: 0.1,
            theta: -0.01,
            rho: 0.01,
            theo: 1.1,
            implied_volatility: Some(0.2),
            contract_specification: None,
        }
    }

    #[test]
    fn catalog_preserves_nullable_market_facts_order_and_contract_count() {
        let contracts = vec![
            contract("MISSING-GAMMA", None, Some(1.0)),
            contract("MISSING-OI", Some(0.01), None),
            contract("BOTH-MISSING", None, None),
            contract("ZERO", Some(0.0), Some(0.0)),
        ];
        let snapshot = Snapshot {
            ticker: "SPY".to_string(),
            timestamp_utc: Utc.with_ymd_and_hms(2026, 8, 20, 15, 0, 0).unwrap(),
            contratos: contracts.clone(),
            chains: vec![OptionChain {
                root: "SPY".to_string(),
                contratos: contracts,
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: Default::default(),
        };

        let result = catalog("SPY", &snapshot, 100.0);
        assert_eq!(result.contracts.len(), 4);
        assert_eq!(
            result
                .contracts
                .iter()
                .map(|contract| contract.occ_symbol.as_str())
                .collect::<Vec<_>>(),
            ["MISSING-GAMMA", "MISSING-OI", "BOTH-MISSING", "ZERO"]
        );
        assert_eq!(result.contracts[0].gamma, None);
        assert_eq!(result.contracts[0].open_interest, Some(1.0));
        assert_eq!(result.contracts[1].gamma, Some(0.01));
        assert_eq!(result.contracts[1].open_interest, None);
        assert_eq!(result.contracts[2].gamma, None);
        assert_eq!(result.contracts[2].open_interest, None);
        assert_eq!(result.contracts[3].gamma, Some(0.0));
        assert_eq!(result.contracts[3].open_interest, Some(0.0));

        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["contracts"][0]["gamma"], serde_json::Value::Null);
        assert_eq!(
            json["contracts"][1]["open_interest"],
            serde_json::Value::Null
        );
        assert_eq!(json["contracts"][3]["gamma"], 0.0);
        assert_eq!(json["contracts"][3]["open_interest"], 0.0);
    }
}
