//! One-off, idempotent migration of yield curves to DuckDB.

use std::error::Error;

use hexagonal_backend::{
    configurator::{configure_yield_curve_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_yield_curves::ForMigratingYieldCurves,
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=ro")
        .await?;
    initialize_analytical_storage().await?;
    let report = configure_yield_curve_migration(sqlite.clone())
        .migrate_yield_curves()
        .await?;
    println!("Curvas SQLite: {}", report.source_rows);
    println!("Curvas DuckDB: {}", report.target_rows);
    sqlite.close().await;
    Ok(())
}
