//! Conversation required to obtain risk-free yield curves.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::treasury::YieldCurve};

#[async_trait]
pub trait ForObtainingYieldCurves: Send + Sync {
    async fn obtain_yield_curves(&self, year: i32) -> PortResult<Vec<YieldCurve>>;
}
