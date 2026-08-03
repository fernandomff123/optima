use polars_options::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;
use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/polars_options.db?mode=rwc")
        .await?;

    let configured = polars_options::configurator::configure(pool.clone());
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let tickers: Vec<_> = if arguments.is_empty() {
        vec!["VIX".to_string()]
    } else {
        arguments
    };
    let mut succeeded = 0;
    let mut inserted = 0;
    let mut failures = Vec::new();
    for ticker in &tickers {
        match configured
            .synchronization
            .synchronize_volatility_index(ticker)
            .await
        {
            Ok(report) => {
                succeeded += 1;
                inserted += report.items_stored;
            }
            Err(error) => failures.push((ticker, error)),
        }
    }

    println!("Índices pedidos: {}", tickers.len());
    println!("Índices sincronizados: {succeeded}");
    println!("Linhas inseridas ou atualizadas: {inserted}");
    println!("Falhas: {}", failures.len());
    for (ticker, error) in failures {
        println!("{ticker}: {error}");
    }

    pool.close().await;
    Ok(())
}
