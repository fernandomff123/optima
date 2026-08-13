use crate::hexagon::{PortResult, domain::data_refresh::DataRefreshRun};
use async_trait::async_trait;

#[async_trait]
pub trait ForLoadingDataRefreshRuns: Send + Sync {
    async fn load_recent_data_refresh_runs(&self, limit: usize) -> PortResult<Vec<DataRefreshRun>>;
    async fn load_running_data_refresh_runs(&self) -> PortResult<Vec<DataRefreshRun>>;
}
