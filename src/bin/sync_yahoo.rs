use polars_options::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;
use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

const TICKERS: [&str; 4] = ["IBM", "GOOGL", "MSFT", "JPM"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/polars_options.db?mode=rwc")
        .await?;
    let configured = polars_options::configurator::configure(pool.clone());
    let since = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).ok_or("invalid initial date")?;

    for ticker in TICKERS {
        match configured
            .synchronization
            .synchronize_market_history(ticker, since)
            .await
        {
            Ok(report) => println!(
                "{ticker}: itens obtidos={}, linhas inseridas ou atualizadas={}",
                report.items_obtained, report.items_stored
            ),
            Err(error) => println!("{ticker}: erro: {error}"),
        }
    }

    pool.close().await;
    Ok(())
}
