//! Conversation offered to actors viewing current option-market data.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::simulation::IntradaySimulationMarket};

#[async_trait]
pub trait ForViewingIntradayOptions: Send + Sync {
    async fn intraday_options(&self, ticker: &str) -> PortResult<IntradaySimulationMarket>;
}
