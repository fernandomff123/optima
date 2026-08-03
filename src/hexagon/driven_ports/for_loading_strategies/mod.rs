//! Conversation required to load saved strategy definitions.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::saved_strategy::SavedStrategy};

#[async_trait]
pub trait ForLoadingStrategies: Send + Sync {
    async fn load_strategies(&self) -> PortResult<Vec<SavedStrategy>>;
}
