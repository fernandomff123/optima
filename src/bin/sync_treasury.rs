use chrono::Datelike;
use hexagonal_backend::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let configured = hexagonal_backend::configurator::configure();
    let current_year = chrono::Utc::now().year();
    let mut succeeded = 0;
    let mut inserted = 0;
    let mut failures = Vec::new();
    for year in 1960..=current_year {
        match configured
            .synchronization
            .synchronize_yield_curves(year)
            .await
        {
            Ok(report) => {
                succeeded += 1;
                inserted += report.items_stored;
            }
            Err(error) => failures.push((year, error)),
        }
    }

    println!("Anos pedidos: {}", current_year - 1959);
    println!("Anos processados: {succeeded}");
    println!("Linhas inseridas ou completadas: {inserted}");
    println!("Falhas: {}", failures.len());
    for (year, error) in failures {
        println!("{year}: {error}");
    }

    Ok(())
}
