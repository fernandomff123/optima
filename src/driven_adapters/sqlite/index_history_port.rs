//! SQLite participant for the index-history loading conversation.

use sqlx::SqlitePool;

use crate::hexagon::{
    PortError, PortResult,
    domain::index_history::IndexHistory,
    driven_ports::{
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_index_history_archive::ForLoadingIndexHistoryArchive,
        for_storing_index_history::ForStoringIndexHistory,
    },
};

#[derive(Clone)]
pub struct SqliteIndexHistoryAdapter {
    pool: SqlitePool,
}

#[async_trait::async_trait]
impl ForLoadingIndexHistoryArchive for SqliteIndexHistoryAdapter {
    async fn load_index_history_archive(&self) -> PortResult<Vec<IndexHistory>> {
        super::index_history::load_all_histories(&self.pool)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}

impl SqliteIndexHistoryAdapter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ForLoadingIndexHistory for SqliteIndexHistoryAdapter {
    async fn load_index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        super::index_history::load_history(&self.pool, ticker)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}

#[async_trait::async_trait]
impl ForStoringIndexHistory for SqliteIndexHistoryAdapter {
    async fn store_index_history(&self, history: &IndexHistory) -> PortResult<u64> {
        super::index_history::insert_history(&self.pool, history)
            .await
            .map_err(|error| PortError::Unavailable(error.to_string()))
    }
}
