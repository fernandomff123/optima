use chrono::{Duration, TimeZone, Utc};
use hexagonal_backend::hexagon::{
    application::data_refresh::MarketHistoryBackfillPolicy,
    domain::{
        data_refresh::{DataRefreshFailure, DataRefreshOrigin, DataRefreshRun, DataRefreshState},
        market_history::{DailyQuote, MarketHistory},
    },
};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 13, 22, 0, 0)
        .single()
        .expect("valid fixture")
}

#[test]
fn refresh_state_machine_accepts_all_terminal_outcomes_and_rejects_reentry() {
    for (obtained, failures, expected) in [
        (1, vec![], DataRefreshState::Completed),
        (1, vec![failure()], DataRefreshState::Partial),
        (0, vec![failure()], DataRefreshState::Failed),
    ] {
        let mut run = DataRefreshRun::running(
            "run".into(),
            DataRefreshOrigin::Startup,
            now(),
            now().date_naive(),
        );
        run.finish(now(), obtained, obtained, failures, None)
            .expect("valid transition");
        assert_eq!(run.state, expected);
        assert!(run.finish(now(), 0, 0, vec![], None).is_err());
        assert!(run.interrupt(now()).is_err());
    }
}

#[test]
fn every_refresh_origin_is_stable() {
    for origin in [
        DataRefreshOrigin::Startup,
        DataRefreshOrigin::Scheduled,
        DataRefreshOrigin::Retry,
        DataRefreshOrigin::Manual,
    ] {
        let encoded = serde_json::to_string(&origin).expect("serialize");
        assert_eq!(
            serde_json::from_str::<DataRefreshOrigin>(&encoded).expect("deserialize"),
            origin
        );
    }
}

#[test]
fn backfill_is_bounded_incremental_and_skips_current_history() {
    let policy = MarketHistoryBackfillPolicy;
    let target = now().date_naive();
    let empty = history(vec![]);
    assert_eq!(
        policy.since(&empty, target),
        Some(target - Duration::days(45))
    );
    let insufficient = history(
        (0..10)
            .map(|days| quote(target - Duration::days(days)))
            .collect(),
    );
    assert_eq!(
        policy.since(&insufficient, target),
        Some(target - Duration::days(45))
    );
    let current = history(
        (0..22)
            .map(|days| quote(target - Duration::days(days)))
            .collect(),
    );
    assert_eq!(policy.since(&current, target), None);
    let stale = history(
        (1..23)
            .map(|days| quote(target - Duration::days(days)))
            .collect(),
    );
    assert_eq!(policy.since(&stale, target), Some(target));
}
fn failure() -> DataRefreshFailure {
    DataRefreshFailure {
        ticker: "SPY".into(),
        operation: "history".into(),
        error: "controlled".into(),
    }
}
fn quote(date: chrono::NaiveDate) -> DailyQuote {
    DailyQuote {
        timestamp: date.and_hms_opt(20, 0, 0).expect("time").and_utc(),
        open: None,
        high: None,
        low: None,
        close: Some(1.0),
        adjusted_close: None,
        volume: None,
    }
}
fn history(daily_quotes: Vec<DailyQuote>) -> MarketHistory {
    MarketHistory {
        ticker: "SPY".into(),
        currency: None,
        exchange_timezone: None,
        daily_quotes,
        dividends: vec![],
        splits: vec![],
    }
}
