use std::error::Error;

use hexagonal_backend::hexagon::driving_ports::for_streaming_market_prices::ForStreamingMarketPrices;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let ticker = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "AAPL".to_string());
    let quote_count = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3);
    let next_ticker = std::env::args().nth(3);
    let (subscription_updates, subscription) = tokio::sync::watch::channel(ticker);
    let (prices, mut received_prices) = tokio::sync::mpsc::channel(16);
    let application = hexagonal_backend::configurator::configure_market_stream();
    let stream =
        tokio::spawn(async move { application.stream_market_prices(subscription, prices).await });

    for _ in 0..quote_count {
        let Some(price) = received_prices.recv().await else {
            return Err("stream terminou antes de produzir as cotações pedidas".into());
        };
        println!("{price:?}");
    }

    if let Some(next_ticker) = next_ticker {
        subscription_updates.send(next_ticker)?;
        for _ in 0..quote_count {
            let Some(price) = received_prices.recv().await else {
                return Err("stream terminou antes de mudar a subscrição".into());
            };
            println!("{price:?}");
        }
    }

    stream.abort();
    if let Err(error) = stream.await
        && !error.is_cancelled()
    {
        return Err(error.into());
    }
    Ok(())
}
