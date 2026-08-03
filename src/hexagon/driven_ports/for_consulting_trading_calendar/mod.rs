//! Conversation required to consult official exchange sessions.

use chrono::{DateTime, NaiveDate, Utc};

use crate::hexagon::PortResult;

pub trait ForConsultingTradingCalendar: Send + Sync {
    fn is_regular_session(&self, instant: DateTime<Utc>) -> PortResult<bool>;

    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>>;

    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>>;

    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>>;

    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>>;

    fn eligible_session_close(
        &self,
        instant: DateTime<Utc>,
        _delay_minutes: u32,
    ) -> PortResult<Option<DateTime<Utc>>> {
        self.latest_session_close_before(instant).map(Some)
    }

    fn next_end_of_day_attempt(
        &self,
        instant: DateTime<Utc>,
        _delay_minutes: u32,
    ) -> PortResult<DateTime<Utc>> {
        self.next_session_transition(instant)
    }
}
