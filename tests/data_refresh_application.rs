use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use hexagonal_backend::hexagon::{
    PortResult,
    application::data_refresh::DataRefreshApplication,
    domain::{
        data_refresh::{DataRefreshOrigin, DataRefreshRun, DataRefreshState},
        market_history::MarketHistory,
        tracked_ticker::TrackedTicker,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_data_refresh_runs::ForLoadingDataRefreshRuns,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_running_data_refresh_tasks::{DataRefreshTask, ForRunningDataRefreshTasks},
        for_storing_data_refresh_runs::ForStoringDataRefreshRuns,
    },
    driving_ports::{
        for_refreshing_market_data::{
            DataRefreshTrigger, ForRefreshingMarketData, StartDataRefreshResult,
        },
        for_synchronizing_market_data::{
            ForSynchronizingMarketData, SynchronizationReport, SynchronizeTrackedTickers,
            TrackedTickersSynchronizationReport,
        },
    },
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Runs(Mutex<Vec<DataRefreshRun>>);
#[async_trait]
impl ForStoringDataRefreshRuns for Runs {
    async fn store_data_refresh_run(&self, run: &DataRefreshRun) -> PortResult<()> {
        let mut runs = self
            .0
            .lock()
            .map_err(|e| hexagonal_backend::hexagon::PortError::Unavailable(e.to_string()))?;
        if let Some(saved) = runs.iter_mut().find(|saved| saved.id == run.id) {
            *saved = run.clone()
        } else {
            runs.push(run.clone())
        }
        Ok(())
    }
}
#[async_trait]
impl ForLoadingDataRefreshRuns for Runs {
    async fn load_recent_data_refresh_runs(&self, limit: usize) -> PortResult<Vec<DataRefreshRun>> {
        let mut runs = self
            .0
            .lock()
            .map_err(|e| hexagonal_backend::hexagon::PortError::Unavailable(e.to_string()))?
            .clone();
        runs.sort_by_key(|run| std::cmp::Reverse(run.started_at));
        runs.truncate(limit);
        Ok(runs)
    }
    async fn load_running_data_refresh_runs(&self) -> PortResult<Vec<DataRefreshRun>> {
        Ok(self
            .0
            .lock()
            .map_err(|e| hexagonal_backend::hexagon::PortError::Unavailable(e.to_string()))?
            .iter()
            .filter(|run| run.state == DataRefreshState::Running)
            .cloned()
            .collect())
    }
}
struct Tasks(Mutex<Vec<DataRefreshTask>>);
impl Tasks {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }
}
impl ForRunningDataRefreshTasks for Tasks {
    fn run_data_refresh_task(&self, task: DataRefreshTask) {
        if let Ok(mut tasks) = self.0.lock() {
            tasks.push(task)
        }
    }
}
struct ImmediateTasks;
impl ForRunningDataRefreshTasks for ImmediateTasks {
    fn run_data_refresh_task(&self, task: DataRefreshTask) {
        tokio::spawn(task);
    }
}
struct Histories;
#[async_trait]
impl ForLoadingMarketHistory for Histories {
    async fn load_market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        Ok(MarketHistory {
            ticker: ticker.into(),
            currency: None,
            exchange_timezone: None,
            daily_quotes: vec![],
            dividends: vec![],
            splits: vec![],
        })
    }
}
struct Tickers;
#[async_trait]
impl ForLoadingTrackedTickers for Tickers {
    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        Ok(vec![])
    }
}
#[derive(Clone, Copy)]
struct Calendar;
impl ForConsultingTradingCalendar for Calendar {
    fn is_regular_session(&self, _: DateTime<Utc>) -> PortResult<bool> {
        Ok(false)
    }
    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant + Duration::hours(1))
    }
    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant - Duration::hours(1))
    }
    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(14, 30, 0).expect("fixture").and_utc())
    }
    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(21, 0, 0).expect("fixture").and_utc())
    }
    fn next_end_of_day_attempt(&self, instant: DateTime<Utc>, _: u32) -> PortResult<DateTime<Utc>> {
        Ok(instant + Duration::hours(20))
    }
}
struct Sync;
#[async_trait]
impl ForSynchronizingMarketData for Sync {
    async fn synchronize_tracked_tickers(
        &self,
        _: SynchronizeTrackedTickers,
    ) -> PortResult<TrackedTickersSynchronizationReport> {
        unreachable!()
    }
    async fn synchronize_market_history(
        &self,
        _: &str,
        _: NaiveDate,
    ) -> PortResult<SynchronizationReport> {
        Ok(report())
    }
    async fn synchronize_option_chain(
        &self,
        _: &str,
        _: DateTime<Utc>,
    ) -> PortResult<SynchronizationReport> {
        Ok(report())
    }
    async fn synchronize_term_structure(&self, _: &str) -> PortResult<SynchronizationReport> {
        Ok(report())
    }
    async fn synchronize_volatility_index(&self, _: &str) -> PortResult<SynchronizationReport> {
        Ok(report())
    }
    async fn synchronize_yield_curves(&self, _: i32) -> PortResult<SynchronizationReport> {
        Ok(report())
    }
}
fn report() -> SynchronizationReport {
    SynchronizationReport {
        items_obtained: 0,
        items_stored: 0,
    }
}
fn app(runs: Arc<Runs>, tasks: Arc<Tasks>) -> DataRefreshApplication {
    DataRefreshApplication::new(
        Arc::new(Sync),
        runs,
        Arc::new(Histories),
        Arc::new(Tickers),
        Arc::new(Calendar),
        tasks,
    )
}
fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 13, 22, 0, 0)
        .single()
        .expect("fixture")
}

#[tokio::test]
async fn startup_scheduler_and_manual_race_creates_exactly_one_run_and_one_started() {
    let runs = Arc::new(Runs::default());
    let tasks = Arc::new(Tasks::new());
    let application = Arc::new(app(runs.clone(), tasks.clone()));
    let (a, b, c) = tokio::join!(
        application.request_data_refresh(DataRefreshTrigger::Startup, now()),
        application.request_data_refresh(DataRefreshTrigger::Scheduler, now()),
        application.request_data_refresh(DataRefreshTrigger::Manual, now())
    );
    let results = [
        a.expect("startup"),
        b.expect("scheduler"),
        c.expect("manual"),
    ];
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, StartDataRefreshResult::Started(_)))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, StartDataRefreshResult::AlreadyRunning(_)))
            .count(),
        2
    );
    assert_eq!(runs.0.lock().expect("runs").len(), 1);
    assert_eq!(tasks.0.lock().expect("tasks").len(), 1);
}

#[tokio::test]
async fn application_recovers_running_runs_and_restart_schedule_uses_persisted_next_attempt() {
    let runs = Arc::new(Runs::default());
    let tasks = Arc::new(Tasks::new());
    let running = DataRefreshRun::running(
        "interrupted".into(),
        DataRefreshOrigin::Manual,
        now(),
        now().date_naive(),
    );
    runs.store_data_refresh_run(&running).await.expect("seed");
    let application = app(runs.clone(), tasks.clone());
    assert_eq!(
        application
            .recover_interrupted_data_refreshes(now() + Duration::minutes(1))
            .await
            .expect("recover"),
        1
    );
    let recovered = runs
        .load_recent_data_refresh_runs(1)
        .await
        .expect("load")
        .remove(0);
    assert_eq!(recovered.state, DataRefreshState::Failed);
    assert_eq!(
        recovered.next_attempt_at,
        Some(now() + Duration::minutes(1))
    );
    let restarted = app(runs, tasks);
    assert_eq!(
        restarted
            .next_data_refresh_attempt(now())
            .await
            .expect("schedule"),
        now() + Duration::minutes(1)
    );
}

#[tokio::test]
async fn next_attempt_waits_for_the_current_result_and_uses_its_persisted_schedule() {
    let runs = Arc::new(Runs::default());
    let application = DataRefreshApplication::new(
        Arc::new(Sync),
        runs.clone(),
        Arc::new(Histories),
        Arc::new(Tickers),
        Arc::new(Calendar),
        Arc::new(ImmediateTasks),
    );
    let current = Utc::now();
    assert!(matches!(
        application
            .request_data_refresh(DataRefreshTrigger::Scheduler, current)
            .await
            .expect("start"),
        StartDataRefreshResult::Started(_)
    ));
    let next = application
        .next_data_refresh_attempt(current)
        .await
        .expect("next after terminal result");
    let terminal = runs
        .load_recent_data_refresh_runs(1)
        .await
        .expect("terminal")
        .remove(0);
    assert_eq!(terminal.state, DataRefreshState::Completed);
    assert_eq!(terminal.next_attempt_at, Some(next));
    assert!(next > current + Duration::hours(19));
}
