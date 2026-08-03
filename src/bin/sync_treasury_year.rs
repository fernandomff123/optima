use polars_options::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;
use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let year: i32 = std::env::args()
        .nth(1)
        .ok_or("uso: sync_treasury_year <ano>")?
        .parse()?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/polars_options.db?mode=rwc")
        .await?;

    let configured = polars_options::configurator::configure(pool.clone());
    let report = configured
        .synchronization
        .synchronize_yield_curves(year)
        .await?;
    println!(
        "Curvas {year}: obtidas={}, inseridas ou completadas={}",
        report.items_obtained, report.items_stored
    );

    pool.close().await;
    Ok(())
}
