use sqlx::sqlite::SqlitePoolOptions;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite://data/hexagonal.db?mode=rwc")
        .await?;

    hexagonal_backend::configurator::initialize_storage(&pool).await?;

    println!("Migrações de storage concluídas");
    pool.close().await;
    Ok(())
}
