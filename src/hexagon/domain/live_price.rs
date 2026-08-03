//! Technology-neutral live market price.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivePrice {
    pub ticker: String,
    pub price: f64,
    pub market_time: i64,
    pub currency: String,
    pub exchange: String,
    pub regular_session: bool,
    pub change: f64,
    pub change_percent: f64,
    pub day_volume: i64,
}
