//! One-off, idempotent migration of portfolios to DuckDB.
use hexagonal_backend::{
    configurator::{configure_portfolio_migration, initialize_analytical_storage},
    hexagon::driving_ports::for_migrating_portfolios::ForMigratingPortfolios,
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
    let report = configure_portfolio_migration(sqlite.clone())
        .migrate_portfolios()
        .await?;
    println!(
        "SQLite — portfolios: {}, eventos: {}",
        report.source.portfolios, report.source.events
    );
    println!(
        "DuckDB — portfolios: {}, eventos: {}",
        report.target.portfolios, report.target.events
    );
    sqlite.close().await;
    Ok(())
}
