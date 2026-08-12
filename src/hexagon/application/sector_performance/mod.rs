//! Builds sector returns from persisted market histories.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::hexagon::{
    PortResult,
    domain::{
        market_history::{DailyQuote, MarketHistory},
        sector_performance::{
            InstrumentPerformance, PerformanceState, SECTOR_BENCHMARK_TICKER, SECTORS,
            SectorComparison, SectorPerformanceItem, SectorPerformancePeriod,
            SectorPerformanceView, percentage_return,
        },
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_market_history::ForLoadingMarketHistory,
    },
    driving_ports::for_viewing_sector_performance::ForViewingSectorPerformance,
};

pub struct SectorPerformanceApplication<MarketHistory, TradingCalendar> {
    market_history: MarketHistory,
    trading_calendar: TradingCalendar,
}

impl<MarketHistory, TradingCalendar> SectorPerformanceApplication<MarketHistory, TradingCalendar> {
    pub fn new(market_history: MarketHistory, trading_calendar: TradingCalendar) -> Self {
        Self {
            market_history,
            trading_calendar,
        }
    }
}

#[async_trait]
impl<MarketHistoryStore, TradingCalendar> ForViewingSectorPerformance
    for SectorPerformanceApplication<MarketHistoryStore, TradingCalendar>
where
    MarketHistoryStore: ForLoadingMarketHistory,
    TradingCalendar: ForConsultingTradingCalendar,
{
    async fn sector_performance(
        &self,
        period: SectorPerformancePeriod,
        requested_at: DateTime<Utc>,
    ) -> PortResult<SectorPerformanceView> {
        let as_of = self
            .trading_calendar
            .latest_session_close_before(requested_at)?
            .date_naive();
        let benchmark_history = self
            .market_history
            .load_market_history(SECTOR_BENCHMARK_TICKER)
            .await;
        let benchmark = benchmark_history
            .ok()
            .and_then(|history| trailing_performance(&history, period.sessions(), as_of));

        let mut sectors = Vec::with_capacity(SECTORS.len());
        for sector in SECTORS {
            let comparison = match &benchmark {
                Some(benchmark) => self
                    .market_history
                    .load_market_history(sector.etf)
                    .await
                    .ok()
                    .and_then(|history| {
                        performance_between(&history, benchmark.start_date, benchmark.end_date)
                    })
                    .map(|performance| SectorComparison {
                        relative_strength_percentage_points: performance.return_percent
                            - benchmark.return_percent,
                        sector,
                        performance,
                    })
                    .map_or(PerformanceState::Unavailable, PerformanceState::Available),
                None => PerformanceState::Unavailable,
            };
            sectors.push(SectorPerformanceItem { sector, comparison });
        }

        Ok(SectorPerformanceView {
            as_of,
            period,
            benchmark: benchmark.map_or(PerformanceState::Unavailable, PerformanceState::Available),
            sectors,
        })
    }
}

fn trailing_performance(
    history: &MarketHistory,
    sessions: usize,
    as_of: NaiveDate,
) -> Option<InstrumentPerformance> {
    let valid = valid_quotes(history, as_of);
    let end = valid.last()?;
    let start = valid.get(valid.len().checked_sub(sessions + 1)?)?;
    performance(history, start, end)
}

fn performance_between(
    history: &MarketHistory,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Option<InstrumentPerformance> {
    let valid = valid_quotes(history, end_date);
    let start = valid
        .iter()
        .find(|quote| quote.timestamp.date_naive() == start_date)?;
    let end = valid
        .iter()
        .find(|quote| quote.timestamp.date_naive() == end_date)?;
    performance(history, start, end)
}

fn valid_quotes(history: &MarketHistory, as_of: NaiveDate) -> Vec<&DailyQuote> {
    history
        .daily_quotes
        .iter()
        .filter(|quote| {
            quote.timestamp.date_naive() <= as_of
                && closing_price(quote).is_some_and(|close| close.is_finite() && close > 0.0)
        })
        .collect()
}

fn performance(
    history: &MarketHistory,
    start: &DailyQuote,
    end: &DailyQuote,
) -> Option<InstrumentPerformance> {
    Some(InstrumentPerformance {
        ticker: history.ticker.clone(),
        start_date: start.timestamp.date_naive(),
        end_date: end.timestamp.date_naive(),
        return_percent: percentage_return(closing_price(start)?, closing_price(end)?)?,
        observed_at: end.timestamp,
    })
}

fn closing_price(quote: &DailyQuote) -> Option<f64> {
    quote.adjusted_close.or(quote.close)
}
