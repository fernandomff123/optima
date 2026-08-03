//! Persisted option-strategy definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategySide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedStrategyLeg {
    pub occ_symbol: String,
    pub side: StrategySide,
    pub quantity: u32,
    pub entry_price: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedStrategy {
    pub id: i64,
    pub name: String,
    pub ticker: String,
    pub legs: Vec<SavedStrategyLeg>,
    pub updated_at: DateTime<Utc>,
}
