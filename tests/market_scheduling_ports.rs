use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, Utc};
use hexagonal_backend::hexagon::{
    PortResult, application::market_scheduling::MarketSchedulingApplication,
    driven_ports::for_consulting_trading_calendar::ForConsultingTradingCalendar,
    driving_ports::for_scheduling_market_operations::ForSchedulingMarketOperations,
};

#[derive(Default)]
struct CalendarMock {
    delays: Mutex<Vec<u32>>,
}

impl ForConsultingTradingCalendar for CalendarMock {
    fn is_regular_session(&self, _instant: DateTime<Utc>) -> PortResult<bool> {
        Ok(false)
    }

    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant + chrono::Duration::hours(1))
    }

    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant - chrono::Duration::hours(1))
    }

    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(13, 30, 0).expect("valid time").and_utc())
    }

    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(20, 0, 0).expect("valid time").and_utc())
    }

    fn eligible_session_close(
        &self,
        instant: DateTime<Utc>,
        delay_minutes: u32,
    ) -> PortResult<Option<DateTime<Utc>>> {
        self.delays.lock().expect("test mutex").push(delay_minutes);
        Ok(Some(instant))
    }

    fn next_end_of_day_attempt(
        &self,
        instant: DateTime<Utc>,
        delay_minutes: u32,
    ) -> PortResult<DateTime<Utc>> {
        self.delays.lock().expect("test mutex").push(delay_minutes);
        Ok(instant + chrono::Duration::minutes(i64::from(delay_minutes)))
    }
}

#[test]
fn schedules_market_operations_through_a_mocked_calendar() {
    let application = MarketSchedulingApplication::new(CalendarMock::default());
    let now = Utc::now();

    assert_eq!(
        application.eligible_end_of_day_close(now).unwrap(),
        Some(now)
    );
    assert_eq!(
        application.next_end_of_day_attempt(now).unwrap(),
        now + chrono::Duration::minutes(20)
    );
}
