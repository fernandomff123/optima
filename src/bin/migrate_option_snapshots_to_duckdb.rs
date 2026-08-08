//! One-off, idempotent migration from SQLite snapshot BLOBs to DuckDB rows.

use std::error::Error;

use hexagonal_backend::{
    configurator::{configure_option_chain_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_option_chains::ForMigratingOptionChains,
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=ro")
        .await?;
    initialize_analytical_storage().await?;
    let application = configure_option_chain_migration(sqlite.clone());
    let report = application.migrate_option_chains().await?;

    println!("Snapshots SQLite: {}", report.source_snapshots);
    println!("Contratos SQLite: {}", report.source_contracts);
    println!("Snapshots novos no DuckDB: {}", report.inserted_snapshots);
    println!(
        "Snapshots sem market_close ignorados: {}",
        report.skipped_without_market_close
    );
    println!("Snapshots totais no DuckDB: {}", report.target_snapshots);
    println!("Contratos totais no DuckDB: {}", report.target_contracts);

    sqlite.close().await;
    Ok(())
}
