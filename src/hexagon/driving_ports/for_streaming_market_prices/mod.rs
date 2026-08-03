//! Conversation offered to actors subscribing to current market prices.

use async_trait::async_trait;
use tokio::sync::{mpsc, watch};

use crate::hexagon::{PortResult, domain::live_price::LivePrice};

#[async_trait]
pub trait ForStreamingMarketPrices: Send + Sync {
    async fn stream_market_prices(
        &self,
        subscriptions: watch::Receiver<String>,
        prices: mpsc::Sender<LivePrice>,
    ) -> PortResult<()>;
}
