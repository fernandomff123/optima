//! Conversation required to load stored yield curves.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{PortResult, domain::treasury::YieldCurve};

/// Required interface for persistence actors that provide yield curves.
#[async_trait]
pub trait ForLoadingYieldCurves: Send + Sync {
    async fn load_yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>>;
}
