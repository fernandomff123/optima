use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedTicker {
    pub ticker: String,
    pub active: bool,
    pub historical_prices: bool,
    pub option_snapshots: bool,
}
