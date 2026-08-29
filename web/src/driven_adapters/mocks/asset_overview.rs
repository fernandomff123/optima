use super::asset_overview_data::{AAPL_PRICES, AAPL_VOLUMES, SPX_PRICES, SPX_VOLUMES};
use crate::{
    domain::asset::{AssetCapability, AssetKind, AssetSymbol},
    ports::asset_overview::{
        AssetOverviewFailure, AssetOverviewPort, AssetOverviewSnapshot, OverviewScenario,
        SnapshotMetric, SnapshotNewsItem, SnapshotRange, SnapshotTable, SnapshotTone,
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
            _ => Ok(Some(match symbol.as_str() {
                "AAPL" => aapl_snapshot(symbol, scenario),
                "SPX" => spx_snapshot(symbol, scenario),
                _ => fallback_snapshot(symbol, scenario),
            })),
        }
    }
}

#[rustfmt::skip]
fn m(label: &'static str, value: Option<&'static str>, unit: Option<&'static str>) -> SnapshotMetric { metric(label, value, None, unit, SnapshotTone::Neutral, true) }
#[rustfmt::skip]
fn ms(label: &'static str, value: Option<&'static str>, suffix: &'static str) -> SnapshotMetric { metric(label, value, Some(suffix), None, SnapshotTone::Neutral, true) }
fn mts(
    label: &'static str,
    value: Option<&'static str>,
    suffix: &'static str,
    tone: SnapshotTone,
) -> SnapshotMetric {
    metric(label, value, Some(suffix), None, tone, true)
}
#[rustfmt::skip]
fn text_with_suffix(label: &'static str, value: &'static str, suffix: &'static str, tone: SnapshotTone) -> SnapshotMetric { metric(label, Some(value), Some(suffix), None, tone, false) }
#[rustfmt::skip]
fn section(label: &'static str) -> SnapshotMetric { metric(label, None, None, None, SnapshotTone::Neutral, false) }
#[rustfmt::skip]
fn metric(label: &'static str, value: Option<&'static str>, suffix: Option<&'static str>, unit: Option<&'static str>, tone: SnapshotTone, numeric: bool) -> SnapshotMetric { SnapshotMetric { label, value, suffix, unit, tone, numeric } }
fn capabilities() -> Vec<AssetCapability> {
    vec![
        AssetCapability::Overview,
        AssetCapability::Chart,
        AssetCapability::Options,
        AssetCapability::Volatility,
        AssetCapability::Gex,
        AssetCapability::Simulation,
    ]
}
fn freshness(scenario: OverviewScenario) -> &'static str {
    if scenario == OverviewScenario::Stale {
        "Updated 18 min ago"
    } else {
        "Updated 2s ago"
    }
}
fn times(step: usize, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| {
            let minute = 570 + index * step;
            format!("{:02}:{:02}", minute / 60, minute % 60)
        })
        .collect()
}

fn aapl_snapshot(symbol: &AssetSymbol, scenario: OverviewScenario) -> AssetOverviewSnapshot {
    let partial = scenario == OverviewScenario::Partial;
    AssetOverviewSnapshot {
        symbol: symbol.clone(),
        kind: AssetKind::Equity,
        name: "Apple Inc.",
        venue: "NASDAQ",
        price: "$191.13",
        absolute_change: "+2.35",
        percentage_change: "+1.24%",
        change_positive: true,
        currency: "USD",
        market_status: "MARKET OPEN",
        observed_at: "13:00:00 ET",
        datetime: "2026-08-28T13:00:00-04:00",
        freshness: freshness(scenario),
        is_stale: scenario == OverviewScenario::Stale,
        is_mock: true,
        capabilities: capabilities(),
        day_range: Some(m("Day Range", Some("188.74 – 192.05"), Some("USD"))),
        chart_times: times(1, 211),
        chart_prices: AAPL_PRICES.to_vec(),
        chart_volumes: AAPL_VOLUMES.to_vec(),
        chart_session_end: "16:00",
        chart_last_price: "191.13",
        chart_last_volume: "842.1K",
        key_statistics: vec![
            m(
                "Market Cap",
                if partial { None } else { Some("2.94T") },
                Some("USD"),
            ),
            m("Shares Outstanding", Some("15.38B"), None),
            m("Float", Some("15.28B"), None),
            m("P/E (TTM)", Some("29.50"), None),
            m("EPS (TTM)", Some("6.48"), Some("USD")),
            m("Beta (5Y Monthly)", Some("1.23"), None),
            m("IV Rank (1Y)", Some("42.3"), None),
            ms("IV Percentile (1Y)", Some("56"), "%"),
        ],
        year_range: Some(SnapshotRange {
            label: "52W Range",
            minimum: "164.08",
            maximum: "199.62",
            position: 0.761,
            insert_after: 6,
            accessible_value: "Current price 191.13 USD within a 52 week range of 164.08 to 199.62 USD",
        }),
        performance: performance("AAPL", "+1.24"),
        earnings: Some(earnings(partial)),
        index_facts: None,
        options_snapshot: options(partial, true),
        latest_news: Some(equity_news()),
    }
}

fn spx_snapshot(symbol: &AssetSymbol, scenario: OverviewScenario) -> AssetOverviewSnapshot {
    let partial = scenario == OverviewScenario::Partial;
    AssetOverviewSnapshot {
        symbol: symbol.clone(),
        kind: AssetKind::Index,
        name: "S&P 500 Index",
        venue: "INDEX",
        price: "5,303.27",
        absolute_change: "+38.12",
        percentage_change: "+0.72%",
        change_positive: true,
        currency: "USD",
        market_status: "MARKET OPEN",
        observed_at: "13:00:00 ET",
        datetime: "2026-08-28T13:00:00-04:00",
        freshness: freshness(scenario),
        is_stale: scenario == OverviewScenario::Stale,
        is_mock: true,
        capabilities: capabilities(),
        day_range: Some(m("Day Range", Some("5,271.44 – 5,315.02"), None)),
        chart_times: times(5, 43),
        chart_prices: SPX_PRICES.to_vec(),
        chart_volumes: SPX_VOLUMES.to_vec(),
        chart_session_end: "16:00",
        chart_last_price: "5,303.27",
        chart_last_volume: "842.1K",
        key_statistics: vec![
            m(
                "Market Cap",
                if partial { None } else { Some("48.31T") },
                Some("USD"),
            ),
            m("Components", Some("503"), None),
            m("P/E (TTM)", Some("29.50"), None),
            m("Dividend Yield", Some("1.31"), Some("%")),
            m("IV Rank (1Y)", Some("42.3"), None),
            ms("IV Percentile (1Y)", Some("56"), "%"),
        ],
        year_range: Some(SnapshotRange {
            label: "52W Range",
            minimum: "4,103.78",
            maximum: "5,669.67",
            position: 0.766,
            insert_after: 4,
            accessible_value: "Current index level 5,303.27 within a 52 week range of 4,103.78 to 5,669.67",
        }),
        performance: performance("SPX", "+0.72"),
        earnings: None,
        index_facts: Some(vec![
            m("Index family", Some("S&P U.S. Indices"), None),
            m("Asset class", Some("Equity"), None),
            m("Weighting", Some("Float-adjusted market cap"), None),
            m("Constituents", Some("503"), None),
            m(
                "Rebalance",
                if partial { None } else { Some("Quarterly") },
                None,
            ),
        ]),
        options_snapshot: options(partial, false),
        latest_news: Some(index_news()),
    }
}

fn fallback_snapshot(symbol: &AssetSymbol, scenario: OverviewScenario) -> AssetOverviewSnapshot {
    let mut snapshot = aapl_snapshot(symbol, scenario);
    snapshot.name = "Mock Equity";
    snapshot.venue = "DEMO";
    snapshot
}

#[rustfmt::skip]
fn performance_tones() -> Vec<Vec<SnapshotTone>> {
    use SnapshotTone::{Negative as N, Neutral as Z, Positive as P};
    vec![vec![Z,P,P,P],vec![Z,N,P,N],vec![Z,P,P,P],vec![Z,P,P,P],vec![Z,P,P,P],vec![Z,P,P,P],vec![Z,P,P,P],vec![Z,P,P,P]]
}

fn performance(symbol: &'static str, daily_change: &'static str) -> SnapshotTable {
    SnapshotTable {
        title: "Performance",
        headings: vec!["Period", symbol, "NASDAQ-100", "S&P 500"],
        rows: vec![
            vec![Some("1D"), Some(daily_change), Some("+0.84"), Some("+0.72")],
            vec![Some("5D"), Some("-1.18"), Some("+0.32"), Some("-0.04")],
            vec![Some("1M"), Some("+4.73"), Some("+6.21"), Some("+3.81")],
            vec![Some("3M"), Some("+12.58"), Some("+14.92"), Some("+10.25")],
            vec![Some("YTD"), Some("+7.68"), Some("+9.41"), Some("+6.15")],
            vec![Some("1Y"), Some("+24.63"), Some("+27.88"), Some("+18.62")],
            vec![
                Some("3Y (Ann.)"),
                Some("+18.44"),
                Some("+16.19"),
                Some("+11.49"),
            ],
            vec![
                Some("5Y (Ann.)"),
                Some("+19.87"),
                Some("+17.23"),
                Some("+15.26"),
            ],
        ],
        tones: performance_tones(),
        suffixes: vec![vec![None, Some("%"), Some("%"), Some("%")]; 8],
        units: vec![vec![None; 4]; 8],
    }
}
fn earnings(partial: bool) -> Vec<SnapshotMetric> {
    vec![
        text_with_suffix(
            "Next Earnings",
            "May 1, 2025",
            "After Market Close",
            SnapshotTone::Special,
        ),
        m("Consensus EPS", Some("1.62"), Some("USD")),
        mts("EPS YoY", Some("+5.88"), "%", SnapshotTone::Positive),
        m(
            "Revenue Est.",
            if partial { None } else { Some("95.35B") },
            Some("USD"),
        ),
        mts("Revenue YoY", Some("+4.15"), "%", SnapshotTone::Positive),
        section("Last Earnings (Jan 30, 2025)"),
        mts("EPS Beat", Some("0.06"), " (3.85%)", SnapshotTone::Positive),
        mts(
            "Revenue Beat",
            Some("1.65B"),
            " (2.09%)",
            SnapshotTone::Positive,
        ),
    ]
}
fn options(partial: bool, include_average: bool) -> Vec<SnapshotMetric> {
    let mut values = vec![
        ms("ATM IV (30D)", Some("25.4"), "%"),
        ms("ATM IV (7D)", Some("23.1"), "%"),
        m("IV Rank (1Y)", Some("42.3"), None),
        ms("IV Percentile (1Y)", Some("56"), "%"),
        m("Put-Call Ratio (Volume)", Some("0.68"), None),
        m("Put-Call Ratio (OI)", Some("0.79"), None),
        m(
            "Total Open Interest",
            if partial { None } else { Some("7.42M") },
            Some("contracts"),
        ),
    ];
    if include_average {
        values.push(m("Avg. Daily Volume (30D)", Some("55.21M"), Some("shares")));
    }
    values
}
#[rustfmt::skip]
fn equity_news() -> Vec<SnapshotNewsItem> { vec![
    SnapshotNewsItem { headline: "Apple Services revenue hits all-time high in fiscal Q2, grows 11% YoY", source: "CNBC", age: "1h ago" },
    SnapshotNewsItem { headline: "Apple reportedly in talks to add Gemini to iPhones", source: "Bloomberg", age: "2h ago" },
    SnapshotNewsItem { headline: "iPhone 17 Pro expected to feature 12GB RAM, per supply chain report", source: "The Verge", age: "3h ago" },
    SnapshotNewsItem { headline: "Apple expands tap-to-pay on iPhone to more European markets", source: "Reuters", age: "4h ago" },
] }
#[rustfmt::skip]
fn index_news() -> Vec<SnapshotNewsItem> { vec![
    SnapshotNewsItem { headline: "Large-cap index advances as market breadth improves", source: "Markets Desk", age: "1h ago" },
    SnapshotNewsItem { headline: "Technology and industrial sectors lead the session", source: "Optima Wire", age: "2h ago" },
    SnapshotNewsItem { headline: "Index volatility eases during afternoon trading", source: "Business Desk", age: "3h ago" },
    SnapshotNewsItem { headline: "Trading volumes remain near the monthly average", source: "Markets Desk", age: "4h ago" },
] }

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    fn load(symbol: &str, scenario: OverviewScenario) -> AssetOverviewSnapshot { MockAssetOverviewAdapter.load(&AssetSymbol::new(symbol), scenario).unwrap().unwrap() }
    fn minutes(time: &str) -> i32 { let (h, m) = time.split_once(':').unwrap(); h.parse::<i32>().unwrap() * 60 + m.parse::<i32>().unwrap() }
    #[test]
    fn fixtures_select_asset_kind_and_optional_panels() {
        let aapl = load("AAPL", OverviewScenario::Normal); let spx = load("SPX", OverviewScenario::Normal);
        assert_eq!(aapl.kind, AssetKind::Equity); assert!(aapl.earnings.is_some() && aapl.index_facts.is_none());
        assert_eq!(spx.kind, AssetKind::Index); assert!(spx.earnings.is_none() && spx.index_facts.is_some());
        let earnings = aapl.earnings.unwrap();
        assert_eq!(earnings.iter().find(|metric| metric.label == "Next Earnings").unwrap().tone, SnapshotTone::Special);
        for label in ["EPS YoY", "Revenue YoY", "EPS Beat", "Revenue Beat"] { assert_eq!(earnings.iter().find(|metric| metric.label == label).unwrap().tone, SnapshotTone::Positive); }
        assert!(aapl.key_statistics.iter().all(|metric| metric.tone == SnapshotTone::Neutral));
        assert_eq!(aapl.options_snapshot.iter().find(|metric| metric.label == "Total Open Interest").unwrap().tone, SnapshotTone::Neutral);
    }
    #[test]
    fn financial_fixtures_define_precision_suffixes_units_and_text_explicitly() {
        let aapl = load("AAPL", OverviewScenario::Normal); let options = &aapl.options_snapshot;
        for (label, value, suffix) in [("ATM IV (30D)", "25.4", "%"), ("ATM IV (7D)", "23.1", "%"), ("IV Percentile (1Y)", "56", "%")] {
            let metric = options.iter().find(|metric| metric.label == label).unwrap(); assert_eq!(metric.value, Some(value)); assert_eq!(metric.suffix, Some(suffix)); assert_eq!(metric.unit, None);
        }
        for (label, value, unit) in [("Total Open Interest", "7.42M", "contracts"), ("Avg. Daily Volume (30D)", "55.21M", "shares")] {
            let metric = options.iter().find(|metric| metric.label == label).unwrap(); assert_eq!(metric.value, Some(value)); assert_eq!(metric.unit, Some(unit)); assert_eq!(metric.suffix, None);
        }
        let earnings = aapl.earnings.unwrap();
        assert_eq!(earnings.len(), 8); assert!(!earnings.iter().any(|metric| metric.label == "Expected Session"));
        let next = earnings.iter().find(|metric| metric.label == "Next Earnings").unwrap(); assert_eq!(next.value, Some("May 1, 2025")); assert_eq!(next.suffix, Some("After Market Close")); assert_eq!(next.tone, SnapshotTone::Special);
        let section = earnings.iter().find(|metric| metric.label == "Last Earnings (Jan 30, 2025)").unwrap(); assert_eq!(section.value, None); assert!(!section.numeric);
        for label in ["EPS Beat", "Revenue Beat"] { let metric = earnings.iter().find(|metric| metric.label == label).unwrap(); assert_eq!(metric.unit, None); assert!(metric.suffix.unwrap().contains('%')); }
    }
    #[test]
    fn aapl_chart_is_dense_aligned_positive_and_stops_at_as_of() {
        let s = load("AAPL", OverviewScenario::Normal); assert_eq!(s.chart_times.len(), 211);
        assert_eq!(s.chart_times.len(), s.chart_prices.len()); assert_eq!(s.chart_times.len(), s.chart_volumes.len());
        assert_eq!(s.chart_times.first().map(String::as_str), Some("09:30")); assert_eq!(s.chart_times.last().map(String::as_str), Some("13:00"));
        assert!(s.chart_times.windows(2).all(|p| minutes(&p[1]) - minutes(&p[0]) == 1));
        assert!(s.chart_prices.iter().chain(s.chart_volumes.iter()).all(|v| v.is_finite() && *v > 0.0));
        assert!(minutes(s.chart_times.last().unwrap()) <= minutes(&s.observed_at[..5])); assert_eq!(s.chart_session_end, "16:00");
        assert_eq!(s.chart_prices.first(), Some(&190.60)); assert_eq!(s.chart_prices[7], 188.70);
        assert_eq!(s.chart_prices.iter().copied().reduce(f64::max), Some(191.72)); assert_eq!(s.chart_prices[185], 191.72);
        assert_eq!(s.chart_prices.last(), Some(&191.13)); assert_eq!(s.chart_volumes.first(), Some(&2.780)); assert_eq!(s.chart_volumes.last(), Some(&0.842));
    }
    #[test]
    fn scenarios_and_fallback_are_deterministic() { assert_eq!(load("AAPL", OverviewScenario::Normal), load("AAPL", OverviewScenario::Normal)); assert_eq!(load("XYZ", OverviewScenario::Normal).name, "Mock Equity"); assert_eq!(OverviewScenario::from_query(Some("other")), OverviewScenario::Normal); }
    #[test]
    fn partial_preserves_chart_and_missing_is_not_zero() { let normal = load("SPX", OverviewScenario::Normal); let partial = load("SPX", OverviewScenario::Partial); assert_eq!(normal.chart_times, partial.chart_times); assert_eq!(normal.chart_prices, partial.chart_prices); assert_eq!(normal.chart_volumes, partial.chart_volumes); assert!(partial.key_statistics[0].value.is_none()); }
}
