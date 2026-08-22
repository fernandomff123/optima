//! Conversation offered to actors viewing the market-volatility overview.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::{
        historical_volatility::HistoricalVolatilityOverview,
        market_volatility::{ImpliedVolatilityOverview, MarketVolatilityOverview},
    },
};

pub const DEFAULT_HISTORICAL_VOLATILITY_HORIZONS: [usize; 3] = [10, 20, 60];
pub const DEFAULT_HISTORICAL_VOLATILITY_SERIES_LIMIT: usize = 252;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoricalVolatilityRequest {
    pub ticker: String,
    pub horizons_sessions: Vec<usize>,
    pub series_limit: usize,
}

impl HistoricalVolatilityRequest {
    pub fn with_defaults(ticker: impl Into<String>) -> Self {
        Self {
            ticker: ticker.into(),
            horizons_sessions: DEFAULT_HISTORICAL_VOLATILITY_HORIZONS.to_vec(),
            series_limit: DEFAULT_HISTORICAL_VOLATILITY_SERIES_LIMIT,
        }
    }
}

#[async_trait]
pub trait ForViewingVolatility: Send + Sync {
    async fn volatility_overview(&self) -> PortResult<MarketVolatilityOverview>;

    async fn historical_volatility(
        &self,
        request: HistoricalVolatilityRequest,
    ) -> PortResult<HistoricalVolatilityOverview>;

    async fn implied_volatility(&self, ticker: &str) -> PortResult<ImpliedVolatilityOverview>;
}
