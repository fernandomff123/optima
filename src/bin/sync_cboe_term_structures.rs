use hexagonal_backend::hexagon::driving_ports::{
    for_managing_tracked_tickers::ForManagingTrackedTickers,
    for_synchronizing_market_data::ForSynchronizingMarketData,
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let configured = hexagonal_backend::configurator::configure();
    let tickers = configured.tracked_tickers.list_tickers(false).await?;
    let requested = tickers
        .iter()
        .filter(|ticker| ticker.active && ticker.option_snapshots)
        .count();
    let mut succeeded = 0;
    let mut inserted_points = 0;
    let mut failures = Vec::new();

    for tracked in tickers
        .into_iter()
        .filter(|ticker| ticker.active && ticker.option_snapshots)
    {
        match configured
            .synchronization
            .synchronize_term_structure(&tracked.ticker)
            .await
        {
            Ok(report) => {
                succeeded += 1;
                inserted_points += report.items_stored;
            }
            Err(error) => failures.push((tracked.ticker, error)),
        }
    }

    println!("Estruturas pedidas: {requested}");
    println!("Estruturas processadas: {succeeded}");
    println!("Pontos inseridos: {inserted_points}");
    println!("Falhas: {}", failures.len());
    for (ticker, error) in failures {
        println!("{ticker}: {error}");
    }
    Ok(())
}
