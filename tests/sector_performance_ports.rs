use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::sector_performance::SectorPerformanceApplication,
    domain::{
        market_history::{DailyQuote, MarketHistory},
        sector_performance::{
            PerformanceState, SECTOR_BENCHMARK_TICKER, SECTORS, SectorPerformancePeriod,
        },
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_market_history::ForLoadingMarketHistory,
    },
    driving_ports::for_viewing_sector_performance::ForViewingSectorPerformance,
};

#[derive(Clone)]
struct Histories(HashMap<String, MarketHistory>);

#[async_trait]
impl ForLoadingMarketHistory for Histories {
    async fn load_market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        self.0
            .get(ticker)
            .cloned()
            .ok_or_else(|| PortError::Unavailable(format!("missing {ticker}")))
    }
}

#[derive(Clone, Copy)]
struct Calendar(DateTime<Utc>);

impl ForConsultingTradingCalendar for Calendar {
    fn is_regular_session(&self, _instant: DateTime<Utc>) -> PortResult<bool> {
        Ok(false)
    }
    fn next_session_transition(&self, _instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(self.0)
    }
    fn latest_session_close_before(&self, _instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(self.0)
    }
    fn session_open(&self, _date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(self.0)
    }
    fn session_close(&self, _date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(self.0)
    }
}

fn history(ticker: &str, prices: &[f64]) -> MarketHistory {
    let first = Utc.with_ymd_and_hms(2026, 1, 2, 21, 0, 0).unwrap();
    MarketHistory {
        ticker: ticker.to_string(),
        currency: Some("USD".to_string()),
        exchange_timezone: Some("America/New_York".to_string()),
        daily_quotes: prices
            .iter()
            .enumerate()
            .map(|(index, close)| DailyQuote {
                timestamp: first + Duration::days(index as i64),
                open: Some(*close),
                high: Some(*close),
                low: Some(*close),
                close: Some(*close),
                adjusted_close: Some(*close),
                volume: Some(1),
            })
            .collect(),
        dividends: Vec::new(),
        splits: Vec::new(),
    }
}

fn complete_histories() -> Histories {
    let prices = (100..=121).map(f64::from).collect::<Vec<_>>();
    let mut histories = HashMap::new();
    histories.insert(
        SECTOR_BENCHMARK_TICKER.to_string(),
        history(SECTOR_BENCHMARK_TICKER, &prices),
    );
    for sector in SECTORS {
        histories.insert(sector.etf.to_string(), history(sector.etf, &prices));
    }
    Histories(histories)
}

#[tokio::test]
async fn calculates_5_10_and_21_session_returns_on_benchmark_dates() {
    let close = Utc.with_ymd_and_hms(2026, 1, 23, 21, 0, 0).unwrap();
    let application = SectorPerformanceApplication::new(complete_histories(), Calendar(close));

    for (period, sessions) in [
        (SectorPerformancePeriod::OneWeek, 5_usize),
        (SectorPerformancePeriod::TwoWeeks, 10),
        (SectorPerformancePeriod::OneMonth, 21),
    ] {
        let view = application
            .sector_performance(period, close + Duration::hours(1))
            .await
            .unwrap();
        let PerformanceState::Available(benchmark) = view.benchmark else {
            panic!("benchmark available")
        };
        let expected = (121.0 / (121.0 - sessions as f64) - 1.0) * 100.0;
        assert!((benchmark.return_percent - expected).abs() < 1e-12);
        assert_eq!(view.sectors.len(), 11);
        for item in view.sectors {
            let PerformanceState::Available(comparison) = item.comparison else {
                panic!("sector available")
            };
            assert_eq!(comparison.performance.start_date, benchmark.start_date);
            assert_eq!(comparison.performance.end_date, benchmark.end_date);
            assert!(comparison.relative_strength_percentage_points.abs() < 1e-12);
        }
    }
}

#[tokio::test]
async fn insufficient_or_invalid_series_are_partial_without_hiding_other_sectors() {
    let close = Utc.with_ymd_and_hms(2026, 1, 23, 21, 0, 0).unwrap();
    let mut histories = complete_histories().0;
    histories.insert("XLF".to_string(), history("XLF", &[100.0, 0.0]));
    let application = SectorPerformanceApplication::new(Histories(histories), Calendar(close));

    let view = application
        .sector_performance(SectorPerformancePeriod::OneWeek, close)
        .await
        .unwrap();
    assert!(matches!(view.benchmark, PerformanceState::Available(_)));
    assert!(matches!(
        view.sectors[0].comparison,
        PerformanceState::Available(_)
    ));
    assert!(matches!(
        view.sectors[1].comparison,
        PerformanceState::Unavailable
    ));
}

#[tokio::test]
async fn unavailable_benchmark_produces_total_unavailability_with_stable_catalog() {
    let close = Utc.with_ymd_and_hms(2026, 1, 23, 21, 0, 0).unwrap();
    let application = SectorPerformanceApplication::new(Histories(HashMap::new()), Calendar(close));

    let view = application
        .sector_performance(SectorPerformancePeriod::OneWeek, close)
        .await
        .unwrap();
    assert!(matches!(view.benchmark, PerformanceState::Unavailable));
    assert_eq!(view.sectors.len(), 11);
    assert!(
        view.sectors
            .iter()
            .all(|item| matches!(item.comparison, PerformanceState::Unavailable))
    );
}
