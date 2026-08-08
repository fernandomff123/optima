//! Conversation required to load underlying reference prices.

use async_trait::async_trait;

use crate::hexagon::PortResult;

#[async_trait]
pub trait ForLoadingReferencePrices: Send + Sync {
    async fn load_reference_price(&self, ticker: &str) -> PortResult<Option<f64>>;
}
