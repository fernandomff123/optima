//! Conversation required to export index histories during offline migration.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::index_history::IndexHistory};

#[async_trait]
pub trait ForLoadingIndexHistoryArchive: Send + Sync {
    async fn load_index_history_archive(&self) -> PortResult<Vec<IndexHistory>>;
}
