//! Conversation used to view S&P 500 sector performance.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{
    PortResult,
    domain::sector_performance::{SectorPerformancePeriod, SectorPerformanceView},
};

#[async_trait]
pub trait ForViewingSectorPerformance: Send + Sync {
    async fn sector_performance(
        &self,
        period: SectorPerformancePeriod,
        requested_at: DateTime<Utc>,
    ) -> PortResult<SectorPerformanceView>;
}
