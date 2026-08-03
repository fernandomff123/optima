use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::hexagon::domain::portfolio::Instrument;

#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentPrice {
    pub price: f64,
    pub currency: String,
    pub source: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuedPosition {
    pub instrument: Instrument,
    pub quantity: Decimal,
    pub market_price: Option<InstrumentPrice>,
    pub market_value: Option<f64>,
}
