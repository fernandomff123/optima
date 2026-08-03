//! Conversation offered to actors managing reusable strategy definitions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::hexagon::{
    PortResult,
    domain::saved_strategy::{SavedStrategy, SavedStrategyLeg},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveStrategy {
    pub name: String,
    pub ticker: String,
    pub legs: Vec<SavedStrategyLeg>,
}

#[async_trait]
pub trait ForManagingSavedStrategies: Send + Sync {
    async fn list_strategies(&self) -> PortResult<Vec<SavedStrategy>>;

    async fn save_strategy(&self, command: SaveStrategy) -> PortResult<SavedStrategy>;

    async fn delete_strategy(&self, id: i64) -> PortResult<()>;
}
