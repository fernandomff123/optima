//! SQLite schema cleanup retained for databases created by older releases.

use sqlx::{Row, SqlitePool};

const RESEARCH_TABLES: &[&str] = &[
    "news_tickers",
    "news",
    "earnings_results",
    "fundamentals",
    "earnings_calendar",
    "analyst_recommendations",
    "earnings_estimates",
    "yahoo_research_sessions",
    "yahoo_research_snapshots",
];

pub async fn remove_research_storage(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    for table in RESEARCH_TABLES {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&mut *transaction)
            .await?;
    }

    let tracked_columns = sqlx::query("PRAGMA table_info(tracked_tickers)")
        .fetch_all(&mut *transaction)
        .await?;
    if tracked_columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "yahoo_research")
    {
        sqlx::query("ALTER TABLE tracked_tickers DROP COLUMN yahoo_research")
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn removes_research_tables_and_tracked_ticker_flag() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE tracked_tickers (
                ticker TEXT PRIMARY KEY,
                yahoo_research INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE TABLE news (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        remove_research_storage(&pool).await.unwrap();

        let news_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'news'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let columns = sqlx::query("PRAGMA table_info(tracked_tickers)")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(news_exists, 0);
        assert!(
            columns
                .iter()
                .all(|row| row.get::<String, _>("name") != "yahoo_research")
        );
    }
}
