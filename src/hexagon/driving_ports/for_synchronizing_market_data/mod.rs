//! Conversation offered to actors that synchronize market data.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::hexagon::PortResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizationReport {
    pub items_obtained: usize,
    pub items_stored: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizeTrackedTickers {
    pub since: NaiveDate,
    pub market_close: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizationFailure {
    pub ticker: String,
    pub operation: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedTickersSynchronizationReport {
    pub tickers: usize,
    pub items_obtained: usize,
    pub items_stored: u64,
    pub failures: Vec<SynchronizationFailure>,
}

/// Provided interface containing the complete synchronization conversation.
#[async_trait]
pub trait ForSynchronizingMarketData: Send + Sync {
    async fn synchronize_tracked_tickers(
        &self,
        request: SynchronizeTrackedTickers,
    ) -> PortResult<TrackedTickersSynchronizationReport>;

    async fn synchronize_market_history(
        &self,
        ticker: &str,
        since: NaiveDate,
    ) -> PortResult<SynchronizationReport>;

    async fn synchronize_option_chain(
        &self,
        ticker: &str,
        market_close: DateTime<Utc>,
    ) -> PortResult<SynchronizationReport>;

    async fn synchronize_term_structure(&self, ticker: &str) -> PortResult<SynchronizationReport>;

    async fn synchronize_volatility_index(&self, ticker: &str)
    -> PortResult<SynchronizationReport>;

    async fn synchronize_yield_curves(&self, year: i32) -> PortResult<SynchronizationReport>;
}
