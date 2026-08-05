//! SQLite participant for the yield-curve loading conversation.

use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::hexagon::{
    PortError, PortResult,
    domain::treasury::YieldCurve,
    driven_ports::{
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
};

#[derive(Clone)]
pub struct SqliteYieldCurvesAdapter {
    pool: SqlitePool,
}

impl SqliteYieldCurvesAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingYieldCurves for SqliteYieldCurvesAdapter {
    async fn load_yield_curve(&self, on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        super::yield_curves::load_latest_on_or_before(&self.pool, on_or_before)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}

#[async_trait::async_trait]
impl ForStoringYieldCurves for SqliteYieldCurvesAdapter {
    async fn store_yield_curves(&self, curves: &[YieldCurve]) -> PortResult<u64> {
        super::yield_curves::insert_curves(&self.pool, curves)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}
