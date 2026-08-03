//! Scheduling decisions based on official market sessions.

use chrono::{DateTime, Utc};

use crate::hexagon::{
    PortResult, driven_ports::for_consulting_trading_calendar::ForConsultingTradingCalendar,
    driving_ports::for_scheduling_market_operations::ForSchedulingMarketOperations,
};

const DAILY_DATA_DELAY_MINUTES: u32 = 20;

#[derive(Debug, Clone)]
pub struct MarketSchedulingApplication<TradingCalendar> {
    trading_calendar: TradingCalendar,
}

impl<TradingCalendar> MarketSchedulingApplication<TradingCalendar> {
    pub fn new(trading_calendar: TradingCalendar) -> Self {
        Self { trading_calendar }
    }
}

impl<TradingCalendar> ForSchedulingMarketOperations for MarketSchedulingApplication<TradingCalendar>
where
    TradingCalendar: ForConsultingTradingCalendar,
{
    fn market_is_open(&self, instant: DateTime<Utc>) -> PortResult<bool> {
        self.trading_calendar.is_regular_session(instant)
    }

    fn next_market_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        self.trading_calendar.next_session_transition(instant)
    }

    fn eligible_end_of_day_close(
        &self,
        instant: DateTime<Utc>,
    ) -> PortResult<Option<DateTime<Utc>>> {
        self.trading_calendar
            .eligible_session_close(instant, DAILY_DATA_DELAY_MINUTES)
    }

    fn next_end_of_day_attempt(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        self.trading_calendar
            .next_end_of_day_attempt(instant, DAILY_DATA_DELAY_MINUTES)
    }
}
