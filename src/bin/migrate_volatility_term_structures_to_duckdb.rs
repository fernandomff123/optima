//! One-off, idempotent migration of volatility term structures to DuckDB.
use hexagonal_backend::{
    configurator::{configure_volatility_term_structure_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_volatility_term_structures::ForMigratingVolatilityTermStructures,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=ro")
        .await?;
    initialize_analytical_storage().await?;
    let report = configure_volatility_term_structure_migration(sqlite.clone())
        .migrate_volatility_term_structures()
        .await?;
    println!("Estruturas: {}", report.structures);
    println!("Pontos SQLite: {}", report.source_points);
    println!("Pontos DuckDB: {}", report.target_points);
    sqlite.close().await;
    Ok(())
}
