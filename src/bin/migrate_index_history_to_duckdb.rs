//! One-off, idempotent migration of volatility-index histories to DuckDB.

use std::error::Error;

use hexagonal_backend::{
    configurator::{configure_index_history_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_index_history::ForMigratingIndexHistory,
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=ro")
        .await?;
    initialize_analytical_storage().await?;
    let report = configure_index_history_migration(sqlite.clone())
        .migrate_index_history()
        .await?;

    println!("Índices migrados: {}", report.indices);
    println!("Linhas SQLite: {}", report.source_rows);
    println!("Linhas DuckDB: {}", report.target_rows);
    sqlite.close().await;
    Ok(())
}
