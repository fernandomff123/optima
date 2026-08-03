//! Conversation required to receive a continuous stream of current prices.

use async_trait::async_trait;
use tokio::sync::{mpsc, watch};

use crate::hexagon::{PortResult, domain::live_price::LivePrice};

#[async_trait]
pub trait ForStreamingLivePrices: Send + Sync {
    async fn stream_live_prices(
        &self,
        subscriptions: watch::Receiver<String>,
        prices: mpsc::Sender<LivePrice>,
    ) -> PortResult<()>;
}
