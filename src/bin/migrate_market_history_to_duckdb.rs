//! One-off, idempotent migration of prices, dividends, and splits to DuckDB.

use std::error::Error;

use hexagonal_backend::{
    configurator::{configure_market_history_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_market_history::ForMigratingMarketHistory,
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=ro")
        .await?;
    initialize_analytical_storage().await?;
    let report = configure_market_history_migration(sqlite.clone())
        .migrate_market_history()
        .await?;

    println!("Ativos migrados: {}", report.histories);
    println!(
        "SQLite — preços: {}, dividendos: {}, splits: {}",
        report.source.prices, report.source.dividends, report.source.splits
    );
    println!(
        "DuckDB — preços: {}, dividendos: {}, splits: {}",
        report.target.prices, report.target.dividends, report.target.splits
    );
    sqlite.close().await;
    Ok(())
}
