use hexagonal_backend::hexagon::driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers;
use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

const DEFAULT_TICKERS: [&str; 5] = ["AAPL", "GOOGL", "IBM", "JPM", "MSFT"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let requested: Vec<String> = std::env::args().skip(1).collect();
    let tickers: Vec<&str> = if requested.is_empty() {
        DEFAULT_TICKERS.to_vec()
    } else {
        requested.iter().map(String::as_str).collect()
    };
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=rwc")
        .await?;
    let configured = hexagonal_backend::configurator::configure(pool.clone());
    let active = configured.tracked_tickers.list_active_tickers().await?;

    for ticker in tickers {
        let normalized = ticker.trim().to_ascii_uppercase();
        let Some(mut tracked) = active
            .iter()
            .find(|tracked| tracked.ticker == normalized)
            .cloned()
        else {
            println!("{normalized}: ticker ativo não encontrado");
            continue;
        };
        tracked.option_snapshots = true;
        configured.tracked_tickers.configure_ticker(tracked).await?;
        println!("{normalized}: option_snapshots ativado");
    }

    pool.close().await;
    Ok(())
}
