use polars_options::hexagon::driving_ports::for_scheduling_market_operations::ForSchedulingMarketOperations;
use polars_options::hexagon::driving_ports::for_synchronizing_market_data::{
    ForSynchronizingMarketData, SynchronizeTrackedTickers,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/polars_options.db?mode=rwc")
        .await?;
    let configured = polars_options::configurator::configure(pool.clone());
    let since = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).ok_or("invalid initial date")?;
    let Some(market_close) = configured
        .market_scheduling
        .eligible_end_of_day_close(chrono::Utc::now())?
    else {
        println!("A sessão atual ainda não é elegível para sincronização em lote");
        return Ok(());
    };
    let report = configured
        .synchronization
        .synchronize_tracked_tickers(SynchronizeTrackedTickers {
            since,
            market_close,
        })
        .await?;

    println!("Tickers ativos: {}", report.tickers);
    println!("Itens obtidos: {}", report.items_obtained);
    println!("Itens inseridos ou atualizados: {}", report.items_stored);
    println!("Falhas: {}", report.failures.len());
    for failure in report.failures {
        println!(
            "{} [{}]: {}",
            failure.ticker, failure.operation, failure.error
        );
    }

    pool.close().await;
    Ok(())
}
