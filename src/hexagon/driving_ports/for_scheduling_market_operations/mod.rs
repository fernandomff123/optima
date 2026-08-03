//! Conversation offered to actors scheduling work around market sessions.

use chrono::{DateTime, Utc};

use crate::hexagon::PortResult;

pub trait ForSchedulingMarketOperations: Send + Sync {
    fn market_is_open(&self, instant: DateTime<Utc>) -> PortResult<bool>;

    fn next_market_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>>;

    fn eligible_end_of_day_close(
        &self,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<DateTime<Utc>>>;

    fn next_end_of_day_attempt(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>>;
}
