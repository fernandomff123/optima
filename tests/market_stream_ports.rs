use async_trait::async_trait;
use hexagonal_backend::hexagon::{
    PortResult, application::market_stream::MarketStreamApplication, domain::live_price::LivePrice,
    driven_ports::for_streaming_live_prices::ForStreamingLivePrices,
    driving_ports::for_streaming_market_prices::ForStreamingMarketPrices,
};

struct LivePriceStreamMock;

#[async_trait]
impl ForStreamingLivePrices for LivePriceStreamMock {
    async fn stream_live_prices(
        &self,
        subscriptions: tokio::sync::watch::Receiver<String>,
        prices: tokio::sync::mpsc::Sender<LivePrice>,
    ) -> PortResult<()> {
        let ticker = subscriptions.borrow().trim().to_ascii_uppercase();
        let _ = prices
            .send(LivePrice {
                ticker,
                price: 100.0,
                market_time: 1,
                currency: "USD".to_string(),
                exchange: "TEST".to_string(),
                regular_session: true,
                change: 1.0,
                change_percent: 1.0,
                day_volume: 10,
            })
            .await;
        Ok(())
    }
}

#[tokio::test]
async fn streams_domain_prices_through_a_mocked_driven_actor() {
    let application = MarketStreamApplication::new(LivePriceStreamMock);
    let (_updates, subscriptions) = tokio::sync::watch::channel(" spx ".to_string());
    let (prices, mut received) = tokio::sync::mpsc::channel(1);

    application
        .stream_market_prices(subscriptions, prices)
        .await
        .unwrap();

    assert_eq!(received.recv().await.unwrap().ticker, "SPX");
}
