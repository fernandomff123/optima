//! Conversation required to export yield curves during offline migration.

use crate::hexagon::{PortResult, domain::treasury::YieldCurve};
use async_trait::async_trait;

#[async_trait]
pub trait ForLoadingYieldCurveArchive: Send + Sync {
    async fn load_yield_curve_archive(&self) -> PortResult<Vec<YieldCurve>>;
}
