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
