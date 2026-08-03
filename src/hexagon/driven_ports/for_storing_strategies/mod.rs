//! Conversation required to store and remove strategy definitions.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::saved_strategy::{SavedStrategy, SavedStrategyLeg},
};

#[async_trait]
pub trait ForStoringStrategies: Send + Sync {
    async fn store_strategy(
        &self,
        name: &str,
        ticker: &str,
        legs: &[SavedStrategyLeg],
    ) -> PortResult<SavedStrategy>;

    /// Returns `true` only when an existing strategy was removed.
    async fn delete_strategy(&self, id: i64) -> PortResult<bool>;
}
