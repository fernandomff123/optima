use crate::hexagon::{PortResult, domain::data_refresh::DataRefreshRun};
use async_trait::async_trait;

#[async_trait]
pub trait ForStoringDataRefreshRuns: Send + Sync {
    async fn store_data_refresh_run(&self, run: &DataRefreshRun) -> PortResult<()>;
}
