//! Conversation offered to actors preparing intraday simulations.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::simulation::IntradaySimulationMarket};

#[async_trait]
pub trait ForPreparingIntradaySimulations: Send + Sync {
    async fn intraday_market(&self, ticker: &str) -> PortResult<IntradaySimulationMarket>;
}
