use crate::hexagon::{PortResult, domain::data_refresh::DataRefreshRun};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartDataRefreshResult {
    Started(DataRefreshRun),
    AlreadyRunning(DataRefreshRun),
    NoEligibleSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRefreshStatus {
    pub running: bool,
    pub latest: Option<DataRefreshRun>,
    pub recent: Vec<DataRefreshRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRefreshTrigger {
    Startup,
    Scheduler,
    Manual,
}

#[async_trait]
pub trait ForRefreshingMarketData: Send + Sync {
    async fn recover_interrupted_data_refreshes(&self, now: DateTime<Utc>) -> PortResult<u64>;

    async fn request_data_refresh(
        &self,
        trigger: DataRefreshTrigger,
        now: DateTime<Utc>,
    ) -> PortResult<StartDataRefreshResult>;
    async fn next_data_refresh_attempt(&self, now: DateTime<Utc>) -> PortResult<DateTime<Utc>>;
    async fn data_refresh_status(&self, recent_limit: usize) -> PortResult<DataRefreshStatus>;
}
