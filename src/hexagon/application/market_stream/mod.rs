//! Live market-price streaming use case.

use async_trait::async_trait;
use tokio::sync::{mpsc, watch};

use crate::hexagon::{
    PortResult, domain::live_price::LivePrice,
    driven_ports::for_streaming_live_prices::ForStreamingLivePrices,
    driving_ports::for_streaming_market_prices::ForStreamingMarketPrices,
};

pub struct MarketStreamApplication<LivePriceStream> {
    live_price_stream: LivePriceStream,
}

impl<LivePriceStream> MarketStreamApplication<LivePriceStream> {
    pub fn new(live_price_stream: LivePriceStream) -> Self {
        Self { live_price_stream }
    }
}

#[async_trait]
impl<LivePriceStream> ForStreamingMarketPrices for MarketStreamApplication<LivePriceStream>
where
    LivePriceStream: ForStreamingLivePrices,
{
    async fn stream_market_prices(
        &self,
        subscriptions: watch::Receiver<String>,
        prices: mpsc::Sender<LivePrice>,
    ) -> PortResult<()> {
        self.live_price_stream
            .stream_live_prices(subscriptions, prices)
            .await
    }
}
