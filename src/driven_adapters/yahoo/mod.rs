//! Driven adapter for the external historical-market-data actor.

mod client;
pub mod live;
mod parser;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortError, PortResult,
    domain::{live_price::LivePrice, market_history::MarketHistory},
    driven_ports::{
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_streaming_live_prices::ForStreamingLivePrices,
    },
};

/// Obtains historical prices and corporate actions from Yahoo Finance.
///
/// The provider name belongs to this adapter. The port implemented here is
/// deliberately provider-neutral and describes only what the application needs.
#[derive(Debug, Default, Clone, Copy)]
pub struct YahooMarketHistoryAdapter;

/// Obtains a current price from Yahoo Finance.
#[derive(Debug, Default, Clone, Copy)]
pub struct YahooLivePricesAdapter;

#[async_trait]
impl ForObtainingMarketHistory for YahooMarketHistoryAdapter {
    async fn obtain_market_history(
        &self,
        ticker: &str,
        since: NaiveDate,
    ) -> PortResult<MarketHistory> {
        let ticker = ticker.trim().to_ascii_uppercase();
        let provider_symbol = match ticker.as_str() {
            "SPX" => "^GSPC",
            _ => &ticker,
        };

        let response = client::download_historico(provider_symbol, since)
            .await
            .map_err(unavailable)?;
        let mut history = parser::response_to_market_history(response).map_err(unavailable)?;

        // Keep provider-specific aliases from leaking into the domain model.
        history.ticker = ticker;
        Ok(history)
    }
}

#[async_trait]
impl ForObtainingLivePrices for YahooLivePricesAdapter {
    async fn obtain_live_price(&self, ticker: &str) -> PortResult<LivePrice> {
        let price = live::fetch_price(ticker).await.map_err(unavailable)?;
        Ok(LivePrice {
            ticker: price.ticker,
            price: price.price,
            market_time: price.time,
            currency: price.currency,
            exchange: price.exchange,
            regular_session: price.market_hours == 1,
            change: price.change,
            change_percent: price.change_percent,
            day_volume: price.day_volume,
        })
    }
}

#[async_trait]
impl ForStreamingLivePrices for YahooLivePricesAdapter {
    async fn stream_live_prices(
        &self,
        subscriptions: tokio::sync::watch::Receiver<String>,
        prices: tokio::sync::mpsc::Sender<LivePrice>,
    ) -> PortResult<()> {
        let (provider_prices, mut received_provider_prices) = tokio::sync::mpsc::channel(32);
        let stream = live::stream_prices(subscriptions, provider_prices);
        let forward = async move {
            while let Some(price) = received_provider_prices.recv().await {
                if prices
                    .send(LivePrice {
                        ticker: price.ticker,
                        price: price.price,
                        market_time: price.time,
                        currency: price.currency,
                        exchange: price.exchange,
                        regular_session: price.market_hours == 1,
                        change: price.change,
                        change_percent: price.change_percent,
                        day_volume: price.day_volume,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };
        tokio::try_join!(async { stream.await.map_err(unavailable) }, async {
            forward.await;
            Ok::<(), PortError>(())
        })?;
        Ok(())
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}

pub(crate) use client::{download_atualizacao, normalize_ticker};
