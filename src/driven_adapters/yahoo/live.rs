//! Yahoo HTTP/WebSocket implementation for current prices.

use std::{error::Error, time::Duration};

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use serde::Deserialize;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

const YAHOO_STREAM_URL: &str = "wss://streamer.finance.yahoo.com/?version=2";

#[derive(Debug, Clone, PartialEq)]
pub struct YahooLivePrice {
    pub ticker: String,
    pub price: f64,
    pub time: i64,
    pub currency: String,
    pub exchange: String,
    pub market_hours: i32,
    pub change: f64,
    pub change_percent: f64,
    pub day_volume: i64,
}

pub async fn fetch_price(ticker: &str) -> Result<YahooLivePrice, Box<dyn Error + Send + Sync>> {
    let ticker = crate::driven_adapters::yahoo::normalize_ticker(ticker)?;
    let response = crate::driven_adapters::yahoo::download_atualizacao(
        &stream_symbol(&ticker),
        chrono::Utc::now().date_naive(),
    )
    .await?;
    let result = response
        .chart
        .result
        .and_then(|mut results| results.pop())
        .ok_or("Yahoo não devolveu uma cotação")?;
    let price = result
        .meta
        .regular_market_price
        .ok_or("Yahoo não devolveu o preço atual")?;
    let previous_close = result.meta.chart_previous_close;
    let change = previous_close.map_or(0.0, |previous| price - previous);
    let change_percent = previous_close
        .filter(|previous| *previous != 0.0)
        .map_or(0.0, |previous| change / previous * 100.0);
    // Session state is Yahoo data. Exchange-calendar decisions belong to the
    // application and must not couple this adapter to another technology.
    let market_hours = i32::from(
        result
            .meta
            .market_state
            .as_deref()
            .is_some_and(|state| state.eq_ignore_ascii_case("REGULAR")),
    );
    Ok(YahooLivePrice {
        ticker,
        price,
        time: result.meta.regular_market_time.unwrap_or_default(),
        currency: result.meta.currency.unwrap_or_default(),
        exchange: result.meta.exchange_name.unwrap_or_default(),
        market_hours,
        change,
        change_percent,
        day_volume: result.meta.regular_market_volume.unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
struct YahooStreamEnvelope {
    message: String,
}

#[derive(Clone, PartialEq, ProstMessage)]
struct PricingData {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(float, tag = "2")]
    price: f32,
    #[prost(sint64, tag = "3")]
    time: i64,
    #[prost(string, tag = "4")]
    currency: String,
    #[prost(string, tag = "5")]
    exchange: String,
    #[prost(int32, tag = "7")]
    market_hours: i32,
    #[prost(float, tag = "8")]
    change_percent: f32,
    #[prost(sint64, tag = "9")]
    day_volume: i64,
    #[prost(float, tag = "12")]
    change: f32,
}

pub async fn stream_prices(
    mut subscription: watch::Receiver<String>,
    prices: mpsc::Sender<YahooLivePrice>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut ticker =
        crate::driven_adapters::yahoo::normalize_ticker(&subscription.borrow_and_update())?;
    let mut symbol = stream_symbol(&ticker);
    let mut request = YAHOO_STREAM_URL.into_client_request()?;
    request
        .headers_mut()
        .insert("Origin", "https://finance.yahoo.com".parse()?);
    let (mut socket, _) = connect_async(request).await?;
    socket
        .send(Message::Text(
            serde_json::json!({ "subscribe": [symbol] })
                .to_string()
                .into(),
        ))
        .await?;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
    heartbeat.tick().await;

    loop {
        tokio::select! {
            changed = subscription.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
                let next_ticker =
                    crate::driven_adapters::yahoo::normalize_ticker(
                        &subscription.borrow_and_update(),
                    )?;
                let next_symbol = stream_symbol(&next_ticker);
                if next_symbol == symbol {
                    ticker = next_ticker;
                    continue;
                }
                socket
                    .send(Message::Text(
                        serde_json::json!({ "unsubscribe": [symbol] })
                            .to_string()
                            .into(),
                    ))
                    .await?;
                socket
                    .send(Message::Text(
                        serde_json::json!({ "subscribe": [next_symbol] })
                            .to_string()
                            .into(),
                    ))
                    .await?;
                symbol = next_symbol;
                ticker = next_ticker;
            }
            _ = heartbeat.tick() => {
                socket
                    .send(Message::Text(
                        serde_json::json!({ "subscribe": [symbol] })
                            .to_string()
                            .into(),
                    ))
                    .await?;
            }
            message = socket.next() => {
                match message.ok_or("Yahoo encerrou o stream sem enviar uma mensagem")?? {
                    Message::Text(text) => {
                        let price = decode_price(&text)?;
                        if price.ticker != symbol {
                            continue;
                        }
                        if price.market_hours != 1 {
                            continue;
                        }
                        let price = YahooLivePrice {
                            ticker: ticker.clone(),
                            ..price
                        };
                        if prices.send(price).await.is_err() {
                            return Ok(());
                        }
                    }
                    Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
                    Message::Close(frame) => {
                        return Err(format!("Yahoo encerrou o stream: {frame:?}").into());
                    }
                    _ => {}
                }
            }
        }
    }
}

fn decode_price(text: &str) -> Result<YahooLivePrice, Box<dyn Error + Send + Sync>> {
    let envelope = serde_json::from_str::<YahooStreamEnvelope>(text)?;
    let payload = base64::engine::general_purpose::STANDARD.decode(envelope.message)?;
    let price = PricingData::decode(payload.as_slice())?;
    Ok(YahooLivePrice {
        ticker: price.id,
        price: f64::from(price.price),
        time: price.time,
        currency: price.currency,
        exchange: price.exchange,
        market_hours: price.market_hours,
        change: f64::from(price.change),
        change_percent: f64::from(price.change_percent),
        day_volume: price.day_volume,
    })
}

fn stream_symbol(ticker: &str) -> String {
    match ticker {
        "SPX" => "^GSPC".to_string(),
        "VIX" => "^VIX".to_string(),
        "BRK.B" => "BRK-B".to_string(),
        _ => ticker.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_internal_index_symbols_to_yahoo() {
        assert_eq!(stream_symbol("SPX"), "^GSPC");
        assert_eq!(stream_symbol("VIX"), "^VIX");
        assert_eq!(stream_symbol("BRK.B"), "BRK-B");
        assert_eq!(stream_symbol("AAPL"), "AAPL");
    }
}
