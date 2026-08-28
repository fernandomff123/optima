use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_overview::{
        AssetOverviewFailure, AssetOverviewPort, AssetOverviewSnapshot, OverviewScenario,
        SnapshotMetric, SnapshotTable,
    },
};

#[derive(Default)]
pub struct MockAssetOverviewAdapter;

impl AssetOverviewPort for MockAssetOverviewAdapter {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: OverviewScenario,
    ) -> Result<Option<AssetOverviewSnapshot>, AssetOverviewFailure> {
        match scenario {
            OverviewScenario::Unavailable => Ok(None),
            OverviewScenario::RecoverableError => Err(AssetOverviewFailure::Recoverable),
            OverviewScenario::TerminalError => Err(AssetOverviewFailure::Terminal),
            _ => Ok(Some(snapshot(symbol, scenario))),
        }
    }
}

fn m(
    label: &'static str,
    value: Option<&'static str>,
    unit: Option<&'static str>,
) -> SnapshotMetric {
    SnapshotMetric { label, value, unit }
}

fn snapshot(symbol: &AssetSymbol, scenario: OverviewScenario) -> AssetOverviewSnapshot {
    let partial = scenario == OverviewScenario::Partial;
    AssetOverviewSnapshot {
        symbol: symbol.clone(),
        name: if symbol.as_str() == "SPX" {
            "S&P 500 Index"
        } else {
            "Apple Inc."
        },
        venue: if symbol.as_str() == "SPX" {
            "INDEX"
        } else {
            "NASDAQ"
        },
        price: "5,303.27",
        absolute_change: "+38.12",
        percentage_change: "+0.72%",
        change_positive: true,
        currency: "USD",
        market_status: "MARKET OPEN",
        observed_at: "09:45:31 ET",
        datetime: "2026-08-28T09:45:31-04:00",
        freshness: if scenario == OverviewScenario::Stale {
            "Updated 18 min ago"
        } else {
            "Updated 2s ago"
        },
        is_stale: scenario == OverviewScenario::Stale,
        is_mock: true,
        capabilities: vec![
            AssetCapability::Overview,
            AssetCapability::Chart,
            AssetCapability::Options,
            AssetCapability::Volatility,
            AssetCapability::Gex,
            AssetCapability::Simulation,
        ],
        metrics: vec![
            m("Day range", Some("5,271.44 — 5,315.02"), None),
            m("Volume", Some("842.1K"), None),
            m("52W range", Some("4,103.78 — 5,669.67"), None),
            m("Currency", Some("USD"), None),
        ],
        chart_times: vec![
            "09:30", "09:45", "10:00", "10:30", "11:00", "11:30", "12:00", "12:30", "13:00",
        ],
        chart_prices: vec![
            5278.4, 5265.8, 5286.2, 5291.7, 5284.9, 5298.5, 5306.1, 5314.8, 5303.27,
        ],
        chart_volumes: vec![3.1, 2.4, 1.8, 1.2, 0.9, 1.1, 1.3, 1.0, 0.8421],
        key_statistics: vec![
            m(
                "Market Cap",
                if partial { None } else { Some("48.31T") },
                Some("USD"),
            ),
            m("Components", Some("503"), None),
            m("P/E (TTM)", Some("29.50"), None),
            m("Dividend Yield", Some("1.31%"), None),
            m("Beta (5Y Monthly)", Some("1.00"), None),
            m("IV Rank (1Y)", Some("42.3"), None),
            m("IV Percentile (1Y)", Some("56%"), None),
        ],
        performance: SnapshotTable {
            title: "Performance",
            headings: vec!["Period", "SPX", "NASDAQ-100", "S&P 500"],
            rows: vec![
                vec![Some("1D"), Some("+0.72%"), Some("+0.84%"), Some("+0.72%")],
                vec![Some("5D"), Some("-1.18%"), Some("+0.32%"), Some("-0.04%")],
                vec![Some("1M"), Some("+4.73%"), Some("+6.21%"), Some("+3.81%")],
                vec![
                    Some("3M"),
                    Some("+12.58%"),
                    Some("+14.92%"),
                    Some("+10.25%"),
                ],
                vec![Some("YTD"), Some("+7.68%"), Some("+9.41%"), Some("+6.15%")],
                vec![
                    Some("1Y"),
                    Some("+24.63%"),
                    Some("+27.88%"),
                    Some("+18.62%"),
                ],
            ],
        },
        index_facts: vec![
            m("Index family", Some("S&P U.S. Indices"), None),
            m("Asset class", Some("Equity"), None),
            m("Weighting", Some("Float-adjusted market cap"), None),
            m("Constituents", Some("503"), None),
            m(
                "Rebalance",
                if partial { None } else { Some("Quarterly") },
                None,
            ),
        ],
        options_snapshot: vec![
            m("ATM IV (30D)", Some("25.4%"), None),
            m("ATM IV (7D)", Some("23.1%"), None),
            m("IV Rank (1Y)", Some("42.3"), None),
            m("IV Percentile (1Y)", Some("56%"), None),
            m("Put-Call Ratio (Volume)", Some("0.68"), None),
            m("Put-Call Ratio (OI)", Some("0.79"), None),
            m(
                "Total Open Interest",
                if partial { None } else { Some("7.42M") },
                Some("contracts"),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenarios_are_deterministic_and_unknown_falls_back_to_normal() {
        let adapter = MockAssetOverviewAdapter;
        let symbol = AssetSymbol::new("SPX");
        let first = adapter.load(&symbol, OverviewScenario::Normal).unwrap();
        let second = adapter.load(&symbol, OverviewScenario::Normal).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            OverviewScenario::from_query(Some("other")),
            OverviewScenario::Normal
        );
    }

    #[test]
    fn partial_missing_values_are_not_zero() {
        let snapshot = MockAssetOverviewAdapter
            .load(&AssetSymbol::new("SPX"), OverviewScenario::Partial)
            .unwrap()
            .unwrap();
        assert!(snapshot.key_statistics[0].value.is_none());
        assert!(snapshot.options_snapshot.last().unwrap().value.is_none());
    }

    #[test]
    fn fixture_timestamp_is_fixed_and_semantic() {
        let snapshot = MockAssetOverviewAdapter
            .load(&AssetSymbol::new("SPX"), OverviewScenario::Normal)
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.datetime, "2026-08-28T09:45:31-04:00");
        assert_eq!(snapshot.currency, "USD");
    }
}
