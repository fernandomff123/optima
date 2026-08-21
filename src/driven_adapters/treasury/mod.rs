//! Driven adapter for the external risk-free yield-curve actor.

mod client;
mod parser;

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult, domain::treasury::YieldCurve,
    driven_ports::for_obtaining_yield_curves::ForObtainingYieldCurves,
};

/// Obtains published yield curves from the U.S. Treasury.
#[derive(Debug, Default, Clone, Copy)]
pub struct TreasuryYieldCurvesAdapter;

#[async_trait]
impl ForObtainingYieldCurves for TreasuryYieldCurvesAdapter {
    async fn obtain_yield_curves(&self, year: i32) -> PortResult<Vec<YieldCurve>> {
        let feed = client::download_ano(&year.to_string())
            .await
            .map_err(unavailable)?;
        parser::feed_to_yield_curves(feed).map_err(unavailable)
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}
