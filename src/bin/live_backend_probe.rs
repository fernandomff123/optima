use std::error::Error;

use api_models::AssetLivePrice;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let ticker = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "AAPL".to_string());
    let next_ticker = std::env::args().nth(2);
    let base_url = std::env::var("HEXAGONAL_BACKEND_LIVE_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:3100/api/assets/live".to_string());
    let url = format!("{base_url}?ticker={ticker}");
    let (mut socket, _) = connect_async(url).await?;

    print_next_price(&mut socket).await?;
    if let Some(next_ticker) = next_ticker {
        socket
            .send(Message::Text(
                serde_json::json!({ "ticker": next_ticker })
                    .to_string()
                    .into(),
            ))
            .await?;
        print_next_price(&mut socket).await?;
    }
    socket.close(None).await?;
    Ok(())
}

async fn print_next_price<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(message) = socket.next().await {
        if let Message::Text(text) = message? {
            let price = serde_json::from_str::<AssetLivePrice>(&text)?;
            println!("{price:?}");
            return Ok(());
        }
    }
    Err("backend encerrou o WebSocket antes de enviar uma cotação".into())
}
