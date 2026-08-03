//! Adapter for official exchange-session information.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::America::New_York;
use exchange_calendars_rs::get_calendar;
use std::io;

use crate::hexagon::{
    PortError, PortResult,
    driven_ports::for_consulting_trading_calendar::ForConsultingTradingCalendar,
};

pub(crate) const MARKET: &str = "XNYS";
pub const DAILY_DATA_DELAY_MINUTES: i64 = 20;

/// Consults the configured exchange calendar without exposing its library type.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExchangeTradingCalendarAdapter;

impl ForConsultingTradingCalendar for ExchangeTradingCalendarAdapter {
    fn is_regular_session(&self, instant: DateTime<Utc>) -> PortResult<bool> {
        is_regular_session(instant).map_err(unavailable)
    }

    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        next_regular_session_transition(instant)
            .map_err(unavailable)?
            .ok_or_else(|| PortError::Unavailable("calendar has no later session".to_string()))
    }

    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        calendar()?
            .latest_close_before(&instant)
            .ok_or_else(|| PortError::Unavailable("calendar has no earlier close".to_string()))
    }

    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        let instant = date_at_noon(date)?;
        calendar()?
            .open_time_on_date(&instant)
            .ok_or_else(|| PortError::NotFound(format!("{date} is not a trading session")))
    }

    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        let instant = date_at_noon(date)?;
        calendar()?
            .close_time_on_date(&instant)
            .ok_or_else(|| PortError::NotFound(format!("{date} is not a trading session")))
    }

    fn eligible_session_close(
        &self,
        instant: DateTime<Utc>,
        delay_minutes: u32,
    ) -> PortResult<Option<DateTime<Utc>>> {
        eligible_download_close(instant, i64::from(delay_minutes)).map_err(unavailable)
    }

    fn next_end_of_day_attempt(
        &self,
        instant: DateTime<Utc>,
        delay_minutes: u32,
    ) -> PortResult<DateTime<Utc>> {
        next_eod_attempt(instant, i64::from(delay_minutes))
            .map_err(unavailable)?
            .ok_or_else(|| PortError::Unavailable("calendar has no later session".to_string()))
    }
}

fn calendar() -> PortResult<Box<dyn exchange_calendars_rs::ExchangeCalendar>> {
    get_calendar(MARKET).map_err(|error| unavailable(format!("{error:?}")))
}

fn date_at_noon(date: NaiveDate) -> PortResult<DateTime<Utc>> {
    date.and_hms_opt(12, 0, 0)
        .map(|time| time.and_utc())
        .ok_or_else(|| PortError::InvalidRequest("invalid session date".to_string()))
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}

pub fn eligible_download_close(
    now: DateTime<Utc>,
    delay_minutes: i64,
) -> Result<Option<DateTime<Utc>>, io::Error> {
    let calendar = get_calendar(MARKET)
        .map_err(|_| io::Error::other(format!("calendário {MARKET} não encontrado")))?;
    if !calendar.is_trading_day(&now) {
        return Ok(calendar.latest_close_before(&now));
    }

    let Some(open) = calendar.open_time_on_date(&now) else {
        return Ok(None);
    };
    if now < open {
        return Ok(calendar.latest_close_before(&now));
    }

    let Some(close) = calendar.close_time_on_date(&now) else {
        return Ok(None);
    };
    if now < close + Duration::minutes(delay_minutes) {
        return Ok(None);
    }

    Ok(Some(close))
}

pub fn eligible_download_session_date(now: DateTime<Utc>) -> Result<Option<NaiveDate>, io::Error> {
    Ok(eligible_download_close(now, DAILY_DATA_DELAY_MINUTES)?
        .map(|close| close.with_timezone(&New_York).date_naive()))
}

pub fn session_date(timestamp: DateTime<Utc>) -> NaiveDate {
    timestamp.with_timezone(&New_York).date_naive()
}

pub fn is_regular_session(now: DateTime<Utc>) -> Result<bool, io::Error> {
    let calendar = get_calendar(MARKET)
        .map_err(|_| io::Error::other(format!("calendário {MARKET} não encontrado")))?;
    if !calendar.is_trading_day(&now) {
        return Ok(false);
    }
    let Some(open) = calendar.open_time_on_date(&now) else {
        return Ok(false);
    };
    let Some(close) = calendar.close_time_on_date(&now) else {
        return Ok(false);
    };
    Ok(now >= open && now < close)
}

pub fn next_regular_session_transition(
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, io::Error> {
    let calendar = get_calendar(MARKET)
        .map_err(|_| io::Error::other(format!("calendário {MARKET} não encontrado")))?;
    let (_, calendar_end) = calendar.calendar_bounds();
    let mut candidate = now;
    while candidate <= calendar_end {
        if let Some(open) = calendar.open_time_on_date(&candidate)
            && open > now
        {
            return Ok(Some(open));
        }
        if let Some(close) = calendar.close_time_on_date(&candidate)
            && close > now
        {
            return Ok(Some(close));
        }
        candidate += Duration::days(1);
    }
    Ok(None)
}

pub fn next_eod_attempt(
    now: DateTime<Utc>,
    delay_minutes: i64,
) -> Result<Option<DateTime<Utc>>, io::Error> {
    let calendar = get_calendar(MARKET)
        .map_err(|_| io::Error::other(format!("calendário {MARKET} não encontrado")))?;
    let (_, calendar_end) = calendar.calendar_bounds();
    let mut candidate = now;
    while candidate <= calendar_end {
        if let Some(close) = calendar.close_time_on_date(&candidate) {
            let attempt = close + Duration::minutes(delay_minutes);
            if attempt > now {
                return Ok(Some(attempt));
            }
        }
        candidate += Duration::days(1);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn current_session_is_only_eligible_after_its_delayed_close() {
        let before_delay = Utc.with_ymd_and_hms(2026, 7, 14, 20, 19, 0).unwrap();
        let after_delay = Utc.with_ymd_and_hms(2026, 7, 14, 20, 20, 0).unwrap();

        assert_eq!(
            eligible_download_close(before_delay, DAILY_DATA_DELAY_MINUTES).unwrap(),
            None
        );
        assert_eq!(
            eligible_download_close(after_delay, DAILY_DATA_DELAY_MINUTES).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 14, 20, 0, 0).unwrap())
        );
    }

    #[test]
    fn previous_session_is_eligible_before_market_open() {
        let before_open = Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap();

        assert_eq!(
            eligible_download_close(before_open, DAILY_DATA_DELAY_MINUTES).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 13, 20, 0, 0).unwrap())
        );
    }

    #[test]
    fn previous_session_is_eligible_on_weekends() {
        let saturday = Utc.with_ymd_and_hms(2026, 7, 18, 22, 0, 0).unwrap();

        assert_eq!(
            eligible_download_close(saturday, DAILY_DATA_DELAY_MINUTES).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 17, 20, 0, 0).unwrap())
        );
    }

    #[test]
    fn previous_session_is_eligible_on_market_holidays() {
        let independence_day_observed = Utc.with_ymd_and_hms(2026, 7, 3, 18, 0, 0).unwrap();

        assert_eq!(
            eligible_download_close(independence_day_observed, DAILY_DATA_DELAY_MINUTES).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 2, 20, 0, 0).unwrap())
        );
    }

    #[test]
    fn regular_session_excludes_pre_market_and_market_close() {
        assert!(
            !is_regular_session(Utc.with_ymd_and_hms(2026, 7, 14, 13, 29, 59).unwrap()).unwrap()
        );
        assert!(is_regular_session(Utc.with_ymd_and_hms(2026, 7, 14, 13, 30, 0).unwrap()).unwrap());
        assert!(!is_regular_session(Utc.with_ymd_and_hms(2026, 7, 14, 20, 0, 0).unwrap()).unwrap());
    }

    #[test]
    fn next_transition_moves_from_open_to_close_and_then_to_next_open() {
        let before_open = Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap();
        let during_session = Utc.with_ymd_and_hms(2026, 7, 14, 15, 0, 0).unwrap();
        let after_close = Utc.with_ymd_and_hms(2026, 7, 14, 21, 0, 0).unwrap();

        assert_eq!(
            next_regular_session_transition(before_open).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 14, 13, 30, 0).unwrap())
        );
        assert_eq!(
            next_regular_session_transition(during_session).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 14, 20, 0, 0).unwrap())
        );
        assert_eq!(
            next_regular_session_transition(after_close).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 15, 13, 30, 0).unwrap())
        );
    }

    #[test]
    fn next_eod_attempt_uses_the_next_official_close() {
        let before_close = Utc.with_ymd_and_hms(2026, 7, 14, 19, 0, 0).unwrap();
        let after_window = Utc.with_ymd_and_hms(2026, 7, 14, 20, 30, 0).unwrap();

        assert_eq!(
            next_eod_attempt(before_close, 20).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 14, 20, 20, 0).unwrap())
        );
        assert_eq!(
            next_eod_attempt(after_window, 20).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 7, 15, 20, 20, 0).unwrap())
        );
    }
}
