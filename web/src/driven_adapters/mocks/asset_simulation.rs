use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_simulation::{
        AssetSimulationFailure, AssetSimulationPort, AssetSimulationSnapshot, GreekSnapshot,
        MetricSentiment, PayoffPointSnapshot, PnlHeatmapSnapshot, ScenarioControlSnapshot,
        SimulationLegSnapshot, SimulationMetricSnapshot, SimulationScenario,
    },
};

#[derive(Clone, Copy, Debug, Default)]
pub struct MockAssetSimulationAdapter;

impl AssetSimulationPort for MockAssetSimulationAdapter {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: SimulationScenario,
    ) -> Result<Option<AssetSimulationSnapshot>, AssetSimulationFailure> {
        match scenario {
            SimulationScenario::Unavailable => return Ok(None),
            SimulationScenario::RecoverableError => {
                return Err(AssetSimulationFailure::Recoverable);
            }
            SimulationScenario::Normal | SimulationScenario::Loading => {}
        }
        if symbol.as_str() == "AAPL" {
            Ok(Some(aapl_snapshot(symbol.clone())))
        } else {
            Ok(None)
        }
    }
}

fn aapl_snapshot(symbol: AssetSymbol) -> AssetSimulationSnapshot {
    AssetSimulationSnapshot {
        symbol,
        name: "Apple Inc.",
        venue: "NASDAQ",
        price: "$191.13",
        percentage_change: "+1.24%",
        change_positive: true,
        capabilities: vec![
            AssetCapability::Overview,
            AssetCapability::Chart,
            AssetCapability::Options,
            AssetCapability::Volatility,
            AssetCapability::Gex,
            AssetCapability::Simulation,
        ],
        strategy_name: "Long Call Spread",
        legs: vec![
            SimulationLegSnapshot {
                quantity: 1,
                option_type: "CALL",
                strike: "190",
                expiration: "MAY 17",
                price: "2.80",
            },
            SimulationLegSnapshot {
                quantity: -1,
                option_type: "CALL",
                strike: "200",
                expiration: "MAY 17",
                price: "0.80",
            },
        ],
        payoff: payoff_fixture(),
        current_spot: 191.13,
        breakeven: 192.80,
        current_date: "May 10, 2024",
        expiration_date: "May 17, 2024",
        probability_low: "172.20",
        probability_high: "210.20",
        metrics: vec![
            metric("Max Profit", "+$720", MetricSentiment::Positive),
            metric("Max Loss", "-$280", MetricSentiment::Negative),
            metric("Breakeven", "192.80", MetricSentiment::Special),
            metric("POP", "56%", MetricSentiment::Neutral),
            metric("Net Debit", "$280", MetricSentiment::Neutral),
        ],
        heatmap: heatmap_fixture(),
        greeks: vec![
            greek("Delta", "0.468", "0.047"),
            greek("Gamma", "0.026", "0.003"),
            greek("Vega", "0.154", "0.015"),
            greek("Theta", "-0.084", "-0.008"),
            greek("Rho", "0.055", "0.005"),
        ],
        preset: "Base",
        controls: vec![
            control("Spot", "191.13", "198.00", "160.00", "220.00", 40),
            control("Implied Volatility", "23.8%", "28.0%", "10.0%", "60.0%", 35),
            control("Time", "Today", "+7 days", "Today", "+30 days", 62),
        ],
    }
}

fn payoff_fixture() -> Vec<PayoffPointSnapshot> {
    [
        (158.0, -450.0, -600.0),
        (165.0, -450.0, -600.0),
        (172.0, -450.0, -600.0),
        (180.0, -445.0, -600.0),
        (186.0, -390.0, -600.0),
        (190.0, -170.0, -280.0),
        (192.8, 0.0, 0.0),
        (196.0, 260.0, 320.0),
        (200.0, 610.0, 720.0),
        (203.0, 690.0, 720.0),
        (210.0, 690.0, 720.0),
        (220.0, 690.0, 720.0),
    ]
    .into_iter()
    .map(
        |(underlying_price, current_pnl, expiration_pnl)| PayoffPointSnapshot {
            underlying_price,
            current_pnl,
            expiration_pnl,
        },
    )
    .collect()
}

fn heatmap_fixture() -> PnlHeatmapSnapshot {
    PnlHeatmapSnapshot {
        spot_prices: vec![160.0, 170.0, 180.0, 190.0, 191.13, 200.0, 210.0, 220.0],
        implied_volatilities: vec![10.0, 20.0, 23.8, 30.0, 40.0, 50.0, 60.0],
        values: vec![
            vec![-280.0, -280.0, -280.0, -280.0, -278.0, 720.0, 720.0, 720.0],
            vec![-280.0, -280.0, -205.0, 25.0, 86.0, 720.0, 720.0, 720.0],
            vec![-280.0, -260.0, -132.0, 166.0, 225.0, 637.0, 720.0, 720.0],
            vec![-280.0, -242.0, -88.0, 219.0, 278.0, 552.0, 720.0, 720.0],
            vec![-280.0, -206.0, -24.0, 268.0, 323.0, 445.0, 720.0, 720.0],
            vec![-280.0, -170.0, 28.0, 284.0, 335.0, 367.0, 720.0, 720.0],
            vec![-280.0, -134.0, 76.0, 299.0, 343.0, 305.0, 720.0, 720.0],
        ],
        selected_row: 2,
        selected_column: 4,
    }
}

fn metric(
    label: &'static str,
    value: &'static str,
    sentiment: MetricSentiment,
) -> SimulationMetricSnapshot {
    SimulationMetricSnapshot {
        label,
        value,
        sentiment,
    }
}

fn greek(name: &'static str, value: &'static str, sensitivity: &'static str) -> GreekSnapshot {
    GreekSnapshot {
        name,
        value,
        sensitivity,
    }
}

fn control(
    label: &'static str,
    current: &'static str,
    target: &'static str,
    minimum: &'static str,
    maximum: &'static str,
    position_percent: u8,
) -> ScenarioControlSnapshot {
    ScenarioControlSnapshot {
        label,
        current,
        target,
        minimum,
        maximum,
        position_percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aapl_fixture_is_deterministic_and_grid_dimensions_are_aligned() {
        let snapshot = MockAssetSimulationAdapter
            .load(&AssetSymbol::new("aapl"), SimulationScenario::Normal)
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.strategy_name, "Long Call Spread");
        assert_eq!(snapshot.legs.len(), 2);
        assert_eq!(
            snapshot.heatmap.values.len(),
            snapshot.heatmap.implied_volatilities.len()
        );
        assert!(
            snapshot
                .heatmap
                .values
                .iter()
                .all(|row| row.len() == snapshot.heatmap.spot_prices.len())
        );
        assert_eq!(snapshot.payoff.last().unwrap().expiration_pnl, 720.0);
    }

    #[test]
    fn unsupported_assets_and_explicit_failure_scenarios_are_preserved() {
        let adapter = MockAssetSimulationAdapter;
        assert!(
            adapter
                .load(&AssetSymbol::new("msft"), SimulationScenario::Normal)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            adapter.load(
                &AssetSymbol::new("aapl"),
                SimulationScenario::RecoverableError
            ),
            Err(AssetSimulationFailure::Recoverable)
        );
    }
}
