use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_volatility::{
        AssetVolatilityFailure, AssetVolatilityPort, AssetVolatilitySnapshot,
        VolatilityGridSnapshot, VolatilityHistoryPointSnapshot, VolatilityHistorySummarySnapshot,
        VolatilityMetricSnapshot, VolatilityScenario, VolatilitySmileSnapshot,
        VolatilityTermPointSnapshot,
    },
};

#[derive(Clone, Copy, Debug, Default)]
pub struct MockAssetVolatilityAdapter;

impl AssetVolatilityPort for MockAssetVolatilityAdapter {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: VolatilityScenario,
    ) -> Result<Option<AssetVolatilitySnapshot>, AssetVolatilityFailure> {
        match scenario {
            VolatilityScenario::Unavailable => return Ok(None),
            VolatilityScenario::RecoverableError => {
                return Err(AssetVolatilityFailure::Recoverable);
            }
            VolatilityScenario::Normal | VolatilityScenario::Loading => {}
        }
        if symbol.as_str() == "AAPL" {
            Ok(Some(aapl_snapshot(symbol.clone())))
        } else {
            Ok(None)
        }
    }
}

fn aapl_snapshot(symbol: AssetSymbol) -> AssetVolatilitySnapshot {
    AssetVolatilitySnapshot {
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
        metric: "Implied Volatility",
        option_type: "Calls + Puts",
        expiration_filter: "All",
        normalization: "Moneyness",
        as_of: "Deterministic illustrative fixture",
        grid: grid_fixture(),
        smiles: smile_fixture(),
        term_structure: term_structure_fixture(),
        history: history_fixture(),
        history_summary: VolatilityHistorySummarySnapshot {
            atm_iv_30d: "24.4%",
            rv20: "19.2%",
            rv60: "20.1%",
            iv_rv_spread_30d: "+5.2 vols",
            percentile: "61%",
        },
        snapshot_metrics: vec![
            metric("ATM IV", "23.8%"),
            metric("30D IV", "24.4%"),
            metric("30D RV", "19.2%"),
            metric("IV Rank", "42.3"),
            metric("IV Percentile", "56%"),
            metric("25Δ Skew", "-2.1 vols"),
        ],
    }
}

fn grid_fixture() -> VolatilityGridSnapshot {
    VolatilityGridSnapshot {
        moneyness: vec![0.70, 0.80, 0.90, 1.00, 1.10, 1.20, 1.30],
        days_to_expiry: vec![7, 14, 30, 60, 90, 120, 180, 270, 365],
        implied_volatility_percent: vec![
            vec![31.6, 30.3, 28.7, 27.1, 26.2, 25.6, 24.6, 24.0, 23.3],
            vec![28.4, 27.2, 25.9, 24.6, 23.8, 23.2, 22.3, 21.7, 21.1],
            vec![25.8, 24.6, 23.4, 22.3, 21.6, 21.1, 20.3, 19.7, 19.2],
            vec![24.5, 23.6, 24.4, 22.8, 22.1, 21.5, 20.6, 19.9, 19.3],
            vec![25.4, 24.7, 25.6, 24.0, 23.2, 22.5, 21.6, 20.9, 20.2],
            vec![27.6, 26.8, 27.7, 25.9, 24.9, 24.1, 23.1, 22.2, 21.4],
            vec![30.8, 29.7, 30.4, 28.3, 27.1, 26.0, 24.8, 23.8, 22.9],
        ],
        selected_moneyness_index: 3,
        selected_expiry_index: 2,
    }
}

fn smile_fixture() -> Vec<VolatilitySmileSnapshot> {
    vec![
        smile("7D", 7, [31.6, 28.4, 25.8, 24.5, 25.4, 27.6, 30.8]),
        smile("30D", 30, [28.7, 25.9, 23.4, 24.4, 25.6, 27.7, 30.4]),
        smile("60D", 60, [27.1, 24.6, 22.3, 22.8, 24.0, 25.9, 28.3]),
        smile("90D", 90, [26.2, 23.8, 21.6, 22.1, 23.2, 24.9, 27.1]),
    ]
}

fn smile(
    label: &'static str,
    days_to_expiry: u16,
    implied_volatility_percent: [f64; 7],
) -> VolatilitySmileSnapshot {
    VolatilitySmileSnapshot {
        label,
        days_to_expiry,
        implied_volatility_percent: implied_volatility_percent.into(),
    }
}

fn term_structure_fixture() -> Vec<VolatilityTermPointSnapshot> {
    [
        (7, 24.5),
        (14, 23.6),
        (30, 24.4),
        (60, 22.8),
        (90, 22.1),
        (120, 21.5),
        (180, 20.6),
        (270, 19.9),
        (365, 19.3),
    ]
    .into_iter()
    .map(
        |(days_to_expiry, implied_volatility_percent)| VolatilityTermPointSnapshot {
            days_to_expiry,
            implied_volatility_percent,
        },
    )
    .collect()
}

fn metric(label: &'static str, value: &'static str) -> VolatilityMetricSnapshot {
    VolatilityMetricSnapshot { label, value }
}

fn history_fixture() -> Vec<VolatilityHistoryPointSnapshot> {
    [
        ("2023-06-01", 24.8, 18.6, 16.9, false),
        ("2023-06-12", 23.6, 16.8, 16.4, false),
        ("2023-06-22", 22.9, 15.1, 15.8, false),
        ("2023-07-03", 24.1, 13.2, 15.1, false),
        ("2023-07-14", 26.4, 15.7, 14.8, false),
        ("2023-07-24", 27.2, 18.5, 15.2, true),
        ("2023-08-03", 25.9, 17.6, 16.4, false),
        ("2023-08-14", 24.9, 18.4, 17.5, false),
        ("2023-08-24", 26.1, 19.3, 18.1, false),
        ("2023-09-05", 29.4, 21.5, 19.0, false),
        ("2023-09-15", 31.0, 20.2, 19.4, false),
        ("2023-09-25", 28.6, 18.9, 19.1, false),
        ("2023-10-05", 27.5, 17.7, 18.2, true),
        ("2023-10-16", 25.9, 16.4, 17.6, false),
        ("2023-10-26", 23.8, 15.2, 16.3, false),
        ("2023-11-06", 21.2, 14.1, 15.0, false),
        ("2023-11-16", 20.5, 12.8, 14.2, false),
        ("2023-11-27", 19.6, 11.4, 13.5, false),
        ("2023-12-07", 19.0, 10.2, 12.8, false),
        ("2023-12-18", 21.4, 13.7, 13.4, false),
        ("2023-12-28", 26.8, 17.2, 15.2, false),
        ("2024-01-08", 30.1, 19.2, 16.5, true),
        ("2024-01-18", 27.6, 18.1, 16.8, false),
        ("2024-01-29", 25.1, 16.7, 15.7, false),
        ("2024-02-08", 23.5, 15.8, 14.2, false),
        ("2024-02-20", 22.6, 13.8, 13.4, false),
        ("2024-03-01", 21.4, 11.0, 12.2, false),
        ("2024-03-12", 22.0, 10.6, 11.8, false),
        ("2024-03-22", 20.7, 10.4, 11.4, false),
        ("2024-04-02", 18.9, 10.8, 11.6, false),
        ("2024-04-12", 20.1, 12.4, 12.2, false),
        ("2024-04-23", 22.3, 15.2, 13.8, false),
        ("2024-05-01", 22.9, 17.0, 15.5, false),
        ("2024-05-10", 25.1, 19.8, 18.3, false),
        ("2024-05-20", 24.4, 19.2, 20.1, true),
    ]
    .into_iter()
    .map(
        |(
            label,
            atm_iv_30d_percent,
            realized_volatility_20d_percent,
            realized_volatility_60d_percent,
            earnings,
        )| VolatilityHistoryPointSnapshot {
            label,
            atm_iv_30d_percent,
            realized_volatility_20d_percent,
            realized_volatility_60d_percent,
            earnings,
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aapl_fixture_is_explicit_aligned_and_shared_by_every_view() {
        let snapshot = MockAssetVolatilityAdapter
            .load(&AssetSymbol::new("aapl"), VolatilityScenario::Normal)
            .unwrap()
            .unwrap();
        let grid = &snapshot.grid;
        assert_eq!(grid.implied_volatility_percent.len(), grid.moneyness.len());
        assert!(
            grid.implied_volatility_percent
                .iter()
                .all(|row| row.len() == grid.days_to_expiry.len())
        );
        assert_eq!(grid.implied_volatility_percent[3][2], 24.4);
        assert_eq!(snapshot.snapshot_metrics[1].value, "24.4%");
        assert_eq!(snapshot.history.last().unwrap().atm_iv_30d_percent, 24.4);
        assert!(snapshot.history.last().unwrap().earnings);
        assert_eq!(snapshot.history.len(), 35);
        assert_eq!(snapshot.history_summary.iv_rv_spread_30d, "+5.2 vols");

        for smile in &snapshot.smiles {
            let column = grid
                .days_to_expiry
                .iter()
                .position(|days| *days == smile.days_to_expiry)
                .unwrap();
            assert_eq!(
                smile.implied_volatility_percent,
                grid.implied_volatility_percent
                    .iter()
                    .map(|row| row[column])
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(
            snapshot
                .term_structure
                .iter()
                .map(|point| point.implied_volatility_percent)
                .collect::<Vec<_>>(),
            grid.implied_volatility_percent[3]
        );
    }

    #[test]
    fn unsupported_assets_and_explicit_failure_scenarios_are_preserved() {
        let adapter = MockAssetVolatilityAdapter;
        assert!(
            adapter
                .load(&AssetSymbol::new("msft"), VolatilityScenario::Normal)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            adapter.load(
                &AssetSymbol::new("aapl"),
                VolatilityScenario::RecoverableError
            ),
            Err(AssetVolatilityFailure::Recoverable)
        );
    }
}
