//! One-off, idempotent migration of tracked ticker configuration to DuckDB.
use hexagonal_backend::{
    configurator::{configure_tracked_ticker_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_tracked_tickers::ForMigratingTrackedTickers,
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
    let report = configure_tracked_ticker_migration(sqlite.clone())
        .migrate_tracked_tickers()
        .await?;
    println!("Tickers SQLite: {}", report.source_rows);
    println!("Tickers DuckDB: {}", report.target_rows);
    sqlite.close().await;
    Ok(())
}
