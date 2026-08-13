//! Persistent lifecycle around the existing market-data synchronization conversation.

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Utc};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Mutex, watch};

use crate::hexagon::{
    PortError, PortResult,
    domain::data_refresh::{
        DataRefreshFailure, DataRefreshOrigin, DataRefreshRun, DataRefreshState,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_data_refresh_runs::ForLoadingDataRefreshRuns,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_running_data_refresh_tasks::ForRunningDataRefreshTasks,
        for_storing_data_refresh_runs::ForStoringDataRefreshRuns,
    },
    driving_ports::{
        for_refreshing_market_data::{
            DataRefreshStatus, DataRefreshTrigger, ForRefreshingMarketData, StartDataRefreshResult,
        },
        for_synchronizing_market_data::ForSynchronizingMarketData,
    },
};

const PUBLICATION_DELAY_MINUTES: u32 = 20;
const MINIMUM_VALID_OBSERVATIONS: usize = 22;
const BACKFILL_CALENDAR_DAYS: i64 = 45;
const RETRY_MINUTES: i64 = 5;

/// A bounded initial window supplies 22 sessions plus margin for holidays and missing observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketHistoryBackfillPolicy;

impl MarketHistoryBackfillPolicy {
    pub fn since(
        &self,
        history: &crate::hexagon::domain::market_history::MarketHistory,
        target: chrono::NaiveDate,
    ) -> Option<chrono::NaiveDate> {
        let valid = history
            .daily_quotes
            .iter()
            .filter(|quote| {
                quote.timestamp.date_naive() <= target
                    && (quote.adjusted_close.is_some() || quote.close.is_some())
            })
            .count();
        let latest = history
            .daily_quotes
            .iter()
            .map(|quote| quote.timestamp.date_naive())
            .filter(|date| *date <= target)
            .max();
        if valid >= MINIMUM_VALID_OBSERVATIONS && latest == Some(target) {
            return None;
        }
        if valid < MINIMUM_VALID_OBSERVATIONS {
            Some(target - Duration::days(BACKFILL_CALENDAR_DAYS))
        } else {
            latest
                .map(|date| date + Duration::days(1))
                .or(Some(target - Duration::days(BACKFILL_CALENDAR_DAYS)))
        }
    }
}

pub struct DataRefreshApplication {
    core: Arc<DataRefreshCore>,
    tasks: Arc<dyn ForRunningDataRefreshTasks>,
    start: Mutex<()>,
}

struct DataRefreshCore {
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    runs: Arc<dyn DataRefreshRunRepository>,
    histories: Arc<dyn ForLoadingMarketHistory>,
    tickers: Arc<dyn ForLoadingTrackedTickers>,
    calendar: Arc<dyn ForConsultingTradingCalendar>,
    active: AtomicBool,
    active_changed: watch::Sender<bool>,
    backfill: MarketHistoryBackfillPolicy,
}

pub trait DataRefreshRunRepository: ForStoringDataRefreshRuns + ForLoadingDataRefreshRuns {}
impl<T: ForStoringDataRefreshRuns + ForLoadingDataRefreshRuns> DataRefreshRunRepository for T {}

impl DataRefreshApplication {
    pub fn new(
        synchronization: Arc<dyn ForSynchronizingMarketData>,
        runs: Arc<dyn DataRefreshRunRepository>,
        histories: Arc<dyn ForLoadingMarketHistory>,
        tickers: Arc<dyn ForLoadingTrackedTickers>,
        calendar: Arc<dyn ForConsultingTradingCalendar>,
        tasks: Arc<dyn ForRunningDataRefreshTasks>,
    ) -> Self {
        Self {
            core: Arc::new(DataRefreshCore {
                synchronization,
                runs,
                histories,
                tickers,
                calendar,
                active: AtomicBool::new(false),
                active_changed: watch::channel(false).0,
                backfill: MarketHistoryBackfillPolicy,
            }),
            tasks,
            start: Mutex::new(()),
        }
    }

    pub async fn recover_interrupted(&self, now: DateTime<Utc>) -> PortResult<u64> {
        let mut running = self.core.runs.load_running_data_refresh_runs().await?;
        for run in &mut running {
            run.interrupt(now)?;
            run.next_attempt_at = Some(now);
            self.core.runs.store_data_refresh_run(run).await?;
        }
        self.core.release_active();
        Ok(running.len() as u64)
    }
}

impl DataRefreshCore {
    fn release_active(&self) {
        self.active.store(false, Ordering::Release);
        self.active_changed.send_replace(false);
    }

    async fn execute(
        &self,
        mut run: DataRefreshRun,
        target_close: DateTime<Utc>,
    ) -> PortResult<DataRefreshRun> {
        let target = target_close.date_naive();
        let mut obtained = 0_u64;
        let mut persisted = 0_u64;
        let mut failures = Vec::new();
        let tickers = match self.tickers.load_active_tickers().await {
            Ok(tickers) => tickers,
            Err(error) => {
                failures.push(failure("system", "tracked_tickers", error));
                Vec::new()
            }
        };
        for tracked in tickers {
            if tracked.historical_prices {
                match self.histories.load_market_history(&tracked.ticker).await {
                    Ok(history) => {
                        if let Some(since) = self.backfill.since(&history, target) {
                            match self
                                .synchronization
                                .synchronize_market_history(&tracked.ticker, since)
                                .await
                            {
                                Ok(report) => {
                                    obtained += report.items_obtained as u64;
                                    persisted += report.items_stored;
                                }
                                Err(error) => {
                                    failures.push(failure(&tracked.ticker, "market_history", error))
                                }
                            }
                        }
                    }
                    Err(error) => {
                        failures.push(failure(&tracked.ticker, "load_market_history", error))
                    }
                }
            }
            if tracked.option_snapshots {
                match self
                    .synchronization
                    .synchronize_option_chain(&tracked.ticker, target_close)
                    .await
                {
                    Ok(report) => {
                        obtained += report.items_obtained as u64;
                        persisted += report.items_stored;
                        match self
                            .synchronization
                            .synchronize_term_structure(&tracked.ticker)
                            .await
                        {
                            Ok(report) => {
                                obtained += report.items_obtained as u64;
                                persisted += report.items_stored;
                            }
                            Err(error) => {
                                failures.push(failure(&tracked.ticker, "term_structure", error))
                            }
                        }
                    }
                    Err(error) => failures.push(failure(&tracked.ticker, "option_chain", error)),
                }
            }
        }
        match self
            .synchronization
            .synchronize_volatility_index("VIX")
            .await
        {
            Ok(report) => {
                obtained += report.items_obtained as u64;
                persisted += report.items_stored;
            }
            Err(error) => failures.push(failure("VIX", "volatility_index", error)),
        }
        match self
            .synchronization
            .synchronize_yield_curves(target.year())
            .await
        {
            Ok(report) => {
                obtained += report.items_obtained as u64;
                persisted += report.items_stored;
            }
            Err(error) => failures.push(failure(&target.year().to_string(), "yield_curves", error)),
        }
        let finished_at = Utc::now();
        let next_attempt = if failures.is_empty() {
            Some(
                self.calendar
                    .next_end_of_day_attempt(finished_at, PUBLICATION_DELAY_MINUTES)?,
            )
        } else {
            Some(finished_at + Duration::minutes(RETRY_MINUTES))
        };
        run.finish(finished_at, obtained, persisted, failures, next_attempt)?;
        Ok(run)
    }
}

#[async_trait]
impl ForRefreshingMarketData for DataRefreshApplication {
    async fn recover_interrupted_data_refreshes(&self, now: DateTime<Utc>) -> PortResult<u64> {
        self.recover_interrupted(now).await
    }

    async fn request_data_refresh(
        &self,
        trigger: DataRefreshTrigger,
        now: DateTime<Utc>,
    ) -> PortResult<StartDataRefreshResult> {
        let _start = self.start.lock().await;
        if self
            .core
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            let running = self
                .core
                .runs
                .load_running_data_refresh_runs()
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    PortError::Conflict("a data refresh is already running".to_string())
                })?;
            return Ok(StartDataRefreshResult::AlreadyRunning(running));
        }
        self.core.active_changed.send_replace(true);
        let eligible = self
            .core
            .calendar
            .eligible_session_close(now, PUBLICATION_DELAY_MINUTES);
        let Some(target_close) = (match eligible {
            Ok(close) => close,
            Err(error) => {
                self.core.release_active();
                return Err(error);
            }
        }) else {
            self.core.release_active();
            return Ok(StartDataRefreshResult::NoEligibleSession);
        };
        let origin = match self.origin_for(trigger, now).await {
            Ok(origin) => origin,
            Err(error) => {
                self.core.release_active();
                return Err(error);
            }
        };
        let run = DataRefreshRun::running(
            format!("{}-{}", now.timestamp_micros(), origin_name(origin)),
            origin,
            now,
            target_close.date_naive(),
        );
        if let Err(error) = self.core.runs.store_data_refresh_run(&run).await {
            self.core.release_active();
            return Err(error);
        }
        let core = self.core.clone();
        let task_run = run.clone();
        self.tasks.run_data_refresh_task(Box::pin(async move {
            let terminal = core.execute(task_run.clone(), target_close).await;
            let terminal_attempt = match terminal {
                Ok(run) => core.runs.store_data_refresh_run(&run).await,
                Err(error) => persist_failed_run(&core, task_run.clone(), error).await,
            };
            if let Err(error) = terminal_attempt {
                let _ = persist_failed_run(&core, task_run, error).await;
            }
            core.release_active();
        }));
        Ok(StartDataRefreshResult::Started(run))
    }

    async fn next_data_refresh_attempt(&self, now: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        let mut active_changed = self.core.active_changed.subscribe();
        while self.core.active.load(Ordering::Acquire) {
            active_changed.changed().await.map_err(|_| {
                PortError::Unavailable("data refresh state signal closed".to_string())
            })?;
        }
        if let Some(run) = self
            .core
            .runs
            .load_recent_data_refresh_runs(1)
            .await?
            .into_iter()
            .next()
            && let Some(next) = run.next_attempt_at
        {
            return Ok(next.max(now));
        }
        self.core
            .calendar
            .next_end_of_day_attempt(now, PUBLICATION_DELAY_MINUTES)
    }

    async fn data_refresh_status(&self, recent_limit: usize) -> PortResult<DataRefreshStatus> {
        if recent_limit == 0 {
            return Err(PortError::InvalidRequest(
                "recent limit must be positive".to_string(),
            ));
        }
        let recent = self
            .core
            .runs
            .load_recent_data_refresh_runs(recent_limit)
            .await?;
        let latest = recent.first().cloned();
        Ok(DataRefreshStatus {
            running: latest
                .as_ref()
                .is_some_and(|run| run.state == DataRefreshState::Running),
            latest,
            recent,
        })
    }
}

impl DataRefreshApplication {
    async fn origin_for(
        &self,
        trigger: DataRefreshTrigger,
        now: DateTime<Utc>,
    ) -> PortResult<DataRefreshOrigin> {
        match trigger {
            DataRefreshTrigger::Startup => Ok(DataRefreshOrigin::Startup),
            DataRefreshTrigger::Manual => Ok(DataRefreshOrigin::Manual),
            DataRefreshTrigger::Scheduler => {
                let latest = self
                    .core
                    .runs
                    .load_recent_data_refresh_runs(1)
                    .await?
                    .into_iter()
                    .next();
                Ok(
                    if latest.is_some_and(|run| {
                        matches!(
                            run.state,
                            DataRefreshState::Partial | DataRefreshState::Failed
                        ) && run.next_attempt_at.is_some_and(|next| next <= now)
                    }) {
                        DataRefreshOrigin::Retry
                    } else {
                        DataRefreshOrigin::Scheduled
                    },
                )
            }
        }
    }
}

fn failure(ticker: &str, operation: &str, error: PortError) -> DataRefreshFailure {
    DataRefreshFailure {
        ticker: ticker.to_string(),
        operation: operation.to_string(),
        error: error.to_string(),
    }
}

async fn persist_failed_run(
    core: &DataRefreshCore,
    mut run: DataRefreshRun,
    error: PortError,
) -> PortResult<()> {
    let failed_at = Utc::now();
    run.interrupt(failed_at)?;
    run.summary = format!("Atualização falhou: {error}");
    run.next_attempt_at = Some(failed_at + Duration::minutes(RETRY_MINUTES));
    core.runs.store_data_refresh_run(&run).await
}

fn origin_name(origin: DataRefreshOrigin) -> &'static str {
    match origin {
        DataRefreshOrigin::Startup => "startup",
        DataRefreshOrigin::Scheduled => "scheduled",
        DataRefreshOrigin::Retry => "retry",
        DataRefreshOrigin::Manual => "manual",
    }
}
