use hexagonal_backend::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;
use std::error::Error;

const TICKERS: [&str; 4] = ["IBM", "GOOGL", "MSFT", "JPM"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let configured = hexagonal_backend::configurator::configure();
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

    Ok(())
}
