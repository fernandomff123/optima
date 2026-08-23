//! Conversation offered to actors viewing current option-market data.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::{
        simulation::{IntradaySimulationMarket, SimulationCatalog},
        volatility_surface::VolatilitySurface,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct IntradayOptionsMarket {
    pub market: IntradaySimulationMarket,
    pub catalog: SimulationCatalog,
    pub volatility_surface: Option<VolatilitySurface>,
}

#[async_trait]
pub trait ForViewingIntradayOptions: Send + Sync {
    async fn intraday_options(&self, ticker: &str) -> PortResult<IntradayOptionsMarket>;
}
