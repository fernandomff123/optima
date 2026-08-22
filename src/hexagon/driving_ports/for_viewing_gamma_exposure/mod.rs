//! Conversation offered to actors viewing gamma exposure.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::hexagon::{PortResult, domain::gamma_exposure::GammaExposureAnalysis};

#[derive(Debug, Clone, PartialEq)]
pub struct GammaExposureRequest {
    pub ticker: String,
    pub range_percent: f64,
    pub points: usize,
    pub valuation_time: DateTime<Utc>,
}

#[async_trait]
pub trait ForViewingGammaExposure: Send + Sync {
    async fn gamma_exposure(
        &self,
        request: GammaExposureRequest,
    ) -> PortResult<GammaExposureAnalysis>;
}
