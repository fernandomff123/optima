//! Conversation offered to actors viewing the market-volatility overview.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::{
        historical_volatility::HistoricalVolatilityOverview,
        market_volatility::{ImpliedVolatilityOverview, MarketVolatilityOverview},
    },
};

#[async_trait]
pub trait ForViewingVolatility: Send + Sync {
    async fn volatility_overview(&self) -> PortResult<MarketVolatilityOverview>;

    async fn historical_volatility(&self, ticker: &str)
    -> PortResult<HistoricalVolatilityOverview>;

    async fn implied_volatility(&self, ticker: &str) -> PortResult<ImpliedVolatilityOverview>;
}
