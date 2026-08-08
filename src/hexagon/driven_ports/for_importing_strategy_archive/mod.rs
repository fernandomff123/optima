//! Conversation required to import complete saved-strategy records.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::saved_strategy::SavedStrategy};

#[async_trait]
pub trait ForImportingStrategyArchive: Send + Sync {
    async fn import_strategy(&self, strategy: &SavedStrategy) -> PortResult<()>;
}
