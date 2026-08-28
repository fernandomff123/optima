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
        observed_at: "13:00:00 ET",
        datetime: "2026-08-28T13:00:00-04:00",
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
            "09:30", "09:35", "09:40", "09:45", "09:50", "09:55", "10:00", "10:05", "10:10",
            "10:15", "10:20", "10:25", "10:30", "10:35", "10:40", "10:45", "10:50", "10:55",
            "11:00", "11:05", "11:10", "11:15", "11:20", "11:25", "11:30", "11:35", "11:40",
            "11:45", "11:50", "11:55", "12:00", "12:05", "12:10", "12:15", "12:20", "12:25",
            "12:30", "12:35", "12:40", "12:45", "12:50", "12:55", "13:00",
        ],
        chart_prices: vec![
            5278.40, 5271.85, 5265.80, 5270.25, 5276.90, 5274.10, 5280.65, 5286.20, 5283.75,
            5288.40, 5291.70, 5289.30, 5293.15, 5287.80, 5284.90, 5289.55, 5294.20, 5291.85,
            5296.70, 5298.50, 5301.10, 5299.40, 5303.65, 5306.10, 5304.35, 5308.80, 5306.55,
            5310.20, 5308.10, 5312.45, 5314.80, 5311.60, 5309.25, 5313.40, 5310.75, 5307.90,
            5305.60, 5308.35, 5306.15, 5304.70, 5307.25, 5305.10, 5303.27,
        ],
        chart_volumes: vec![
            3.10, 2.85, 2.62, 2.40, 2.18, 2.02, 1.88, 1.76, 1.65, 1.57, 1.49, 1.42, 1.36, 1.31,
            1.26, 1.22, 1.18, 1.15, 1.12, 1.10, 1.08, 1.06, 1.04, 1.28, 1.02, 1.00, 0.98, 0.96,
            0.94, 1.16, 0.92, 0.90, 0.88, 0.86, 1.05, 0.84, 0.82, 0.80, 0.92, 0.78, 0.76, 0.74,
            0.8421,
        ],
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
        assert_eq!(snapshot.datetime, "2026-08-28T13:00:00-04:00");
        assert_eq!(snapshot.observed_at, "13:00:00 ET");
        assert_eq!(snapshot.currency, "USD");
    }

    #[test]
    fn chart_fixture_has_aligned_five_minute_observations_through_as_of() {
        let adapter = MockAssetOverviewAdapter;
        let symbol = AssetSymbol::new("SPX");
        let normal = adapter
            .load(&symbol, OverviewScenario::Normal)
            .unwrap()
            .unwrap();
        let partial = adapter
            .load(&symbol, OverviewScenario::Partial)
            .unwrap()
            .unwrap();

        assert_eq!(normal.chart_times.len(), 43);
        assert_eq!(normal.chart_prices.len(), 43);
        assert_eq!(normal.chart_volumes.len(), 43);
        assert_eq!(normal.chart_times.first(), Some(&"09:30"));
        assert_eq!(normal.chart_times.last(), Some(&"13:00"));
        assert!(
            normal
                .chart_times
                .windows(2)
                .all(|pair| { minutes(pair[1]) - minutes(pair[0]) == 5 })
        );
        assert!(minutes(normal.chart_times.last().unwrap()) <= minutes(&normal.observed_at[..5]));
        assert!(
            normal
                .chart_prices
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        );
        assert!(
            normal
                .chart_volumes
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        );
        assert_eq!(normal.chart_times, partial.chart_times);
        assert_eq!(normal.chart_prices, partial.chart_prices);
        assert_eq!(normal.chart_volumes, partial.chart_volumes);
    }

    fn minutes(time: &str) -> i32 {
        let (hours, minutes) = time.split_once(':').unwrap();
        hours.parse::<i32>().unwrap() * 60 + minutes.parse::<i32>().unwrap()
    }
}
