//! Persistent lifecycle around the existing market-data synchronization conversation.

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

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
        for_storing_data_refresh_runs::ForStoringDataRefreshRuns,
    },
    driving_ports::{
        for_refreshing_market_data::{
            DataRefreshStatus, ForRefreshingMarketData, StartDataRefreshResult,
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
    synchronization: Arc<dyn ForSynchronizingMarketData>,
    runs: Arc<dyn DataRefreshRunRepository>,
    histories: Arc<dyn ForLoadingMarketHistory>,
    tickers: Arc<dyn ForLoadingTrackedTickers>,
    calendar: Arc<dyn ForConsultingTradingCalendar>,
    execution: Mutex<()>,
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
    ) -> Self {
        Self {
            synchronization,
            runs,
            histories,
            tickers,
            calendar,
            execution: Mutex::new(()),
            backfill: MarketHistoryBackfillPolicy,
        }
    }

    pub async fn recover_interrupted(&self, now: DateTime<Utc>) -> PortResult<u64> {
        self.runs.recover_interrupted_data_refresh_runs(now).await
    }

    async fn execute(
        &self,
        origin: DataRefreshOrigin,
        now: DateTime<Utc>,
        target_close: DateTime<Utc>,
    ) -> PortResult<DataRefreshRun> {
        let target = target_close.date_naive();
        let id = format!("{}-{}", now.timestamp_micros(), origin_name(origin));
        let mut run = DataRefreshRun::running(id, origin, now, target);
        self.runs.store_data_refresh_run(&run).await?;
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
        self.runs.store_data_refresh_run(&run).await?;
        Ok(run)
    }
}

#[async_trait]
impl ForRefreshingMarketData for DataRefreshApplication {
    async fn recover_interrupted_data_refreshes(&self, now: DateTime<Utc>) -> PortResult<u64> {
        self.recover_interrupted(now).await
    }

    fn eligible_data_refresh_session(
        &self,
        now: DateTime<Utc>,
    ) -> PortResult<Option<chrono::NaiveDate>> {
        self.calendar
            .eligible_session_close(now, PUBLICATION_DELAY_MINUTES)
            .map(|close| close.map(|value| value.date_naive()))
    }

    async fn refresh_market_data(
        &self,
        origin: DataRefreshOrigin,
        now: DateTime<Utc>,
    ) -> PortResult<StartDataRefreshResult> {
        let guard = match self.execution.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                let running = self
                    .runs
                    .load_recent_data_refresh_runs(1)
                    .await?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        PortError::Conflict("a data refresh is already running".to_string())
                    })?;
                return Ok(StartDataRefreshResult::AlreadyRunning(running));
            }
        };
        let Some(target_close) = self
            .calendar
            .eligible_session_close(now, PUBLICATION_DELAY_MINUTES)?
        else {
            drop(guard);
            return Ok(StartDataRefreshResult::NoEligibleSession);
        };
        let run = self.execute(origin, now, target_close).await?;
        drop(guard);
        Ok(StartDataRefreshResult::Started(run))
    }

    async fn data_refresh_status(&self, recent_limit: usize) -> PortResult<DataRefreshStatus> {
        if recent_limit == 0 {
            return Err(PortError::InvalidRequest(
                "recent limit must be positive".to_string(),
            ));
        }
        let recent = self
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

fn failure(ticker: &str, operation: &str, error: PortError) -> DataRefreshFailure {
    DataRefreshFailure {
        ticker: ticker.to_string(),
        operation: operation.to_string(),
        error: error.to_string(),
    }
}
fn origin_name(origin: DataRefreshOrigin) -> &'static str {
    match origin {
        DataRefreshOrigin::Startup => "startup",
        DataRefreshOrigin::Scheduled => "scheduled",
        DataRefreshOrigin::Retry => "retry",
        DataRefreshOrigin::Manual => "manual",
    }
}
