use crate::hexagon::{PortResult, domain::data_refresh::DataRefreshRun};
use async_trait::async_trait;

#[async_trait]
pub trait ForStoringDataRefreshRuns: Send + Sync {
    async fn store_data_refresh_run(&self, run: &DataRefreshRun) -> PortResult<()>;
    async fn recover_interrupted_data_refresh_runs(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> PortResult<u64>;
}
