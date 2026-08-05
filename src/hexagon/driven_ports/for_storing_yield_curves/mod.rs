//! Conversation required to persist risk-free yield curves.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::treasury::YieldCurve};

/// Required interface for storing published yield curves.
#[async_trait]
pub trait ForStoringYieldCurves: Send + Sync {
    async fn store_yield_curves(&self, curves: &[YieldCurve]) -> PortResult<u64>;
}
