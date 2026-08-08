//! One-off, idempotent migration of saved strategies to DuckDB.

use hexagonal_backend::{
    configurator::{configure_strategy_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_strategies::ForMigratingStrategies,
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
    let report = configure_strategy_migration(sqlite.clone())
        .migrate_strategies()
        .await?;
    println!(
        "SQLite — estratégias: {}, pernas: {}",
        report.source.strategies, report.source.legs
    );
    println!(
        "DuckDB — estratégias: {}, pernas: {}",
        report.target.strategies, report.target.legs
    );
    sqlite.close().await;
    Ok(())
}
