//! SQLite participant for the yield-curve loading conversation.

use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::hexagon::{
    PortError, PortResult, domain::treasury::YieldCurve,
    driven_ports::for_loading_yield_curves::ForLoadingYieldCurves,
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
