//! Conversation required to obtain current market prices.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::live_price::LivePrice};

#[async_trait]
pub trait ForObtainingLivePrices: Send + Sync {
    async fn obtain_live_price(&self, ticker: &str) -> PortResult<LivePrice>;
}
