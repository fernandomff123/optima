use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    domain::{
        index_history::IndexHistory,
        live_price::LivePrice,
        options::Snapshot,
        portfolio::{Instrument, Portfolio},
        portfolio_valuation::InstrumentPrice,
        treasury::YieldCurve,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_portfolios::ForLoadingPortfolios,
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_obtaining_instrument_prices::ForObtainingInstrumentPrices,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_obtaining_volatility_indices::ForObtainingVolatilityIndices,
        for_obtaining_yield_curves::ForObtainingYieldCurves,
        for_storing_portfolios::ForStoringPortfolios,
    },
};

struct LivePricesMock;

struct InstrumentPricesMock;

#[async_trait]
impl ForObtainingInstrumentPrices for InstrumentPricesMock {
    async fn obtain_instrument_prices(
        &self,
        instruments: &[Instrument],
    ) -> PortResult<Vec<Option<InstrumentPrice>>> {
        Ok(vec![None; instruments.len()])
    }
}

struct MarketHistoryProviderMock;

#[async_trait]
impl ForObtainingMarketHistory for MarketHistoryProviderMock {
    async fn obtain_market_history(
        &self,
        _ticker: &str,
        _since: chrono::NaiveDate,
    ) -> PortResult<hexagonal_backend::hexagon::domain::market_history::MarketHistory> {
        Err(PortError::Unavailable(
            "no configured market history".into(),
        ))
    }
}

#[async_trait]
impl ForObtainingLivePrices for LivePricesMock {
    async fn obtain_live_price(&self, _ticker: &str) -> PortResult<LivePrice> {
        Err(PortError::Unavailable("no configured live price".into()))
    }
}

struct OptionChainsMock;

#[async_trait]
impl ForObtainingOptionChains for OptionChainsMock {
    async fn obtain_option_chain(&self, _ticker: &str) -> PortResult<Snapshot> {
        Err(PortError::Unavailable("no configured option chain".into()))
    }
}

struct VolatilityIndicesMock;

struct IndexHistoryStoreMock;

#[async_trait]
impl ForLoadingIndexHistory for IndexHistoryStoreMock {
    async fn load_index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        Ok(IndexHistory {
            ticker: ticker.to_string(),
            daily_prices: Vec::new(),
        })
    }
}

#[async_trait]
impl ForObtainingVolatilityIndices for VolatilityIndicesMock {
    async fn obtain_volatility_index(&self, _ticker: &str) -> PortResult<IndexHistory> {
        Err(PortError::Unavailable(
            "no configured volatility index".into(),
        ))
    }
}

struct YieldCurvesMock;

struct StoredYieldCurvesMock;

#[async_trait]
impl ForLoadingYieldCurves for StoredYieldCurvesMock {
    async fn load_yield_curve(
        &self,
        _on_or_before: chrono::NaiveDate,
    ) -> PortResult<Option<YieldCurve>> {
        Ok(None)
    }
}

#[async_trait]
impl ForObtainingYieldCurves for YieldCurvesMock {
    async fn obtain_yield_curves(&self, _year: i32) -> PortResult<Vec<YieldCurve>> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct PortfoliosMock(Mutex<HashMap<String, Portfolio>>);

#[async_trait]
impl ForStoringPortfolios for PortfoliosMock {
    async fn store_portfolio(&self, portfolio: &Portfolio) -> PortResult<()> {
        self.0
            .lock()
            .expect("test mutex must be usable")
            .insert(portfolio.id.clone(), portfolio.clone());
        Ok(())
    }
}

#[async_trait]
impl ForLoadingPortfolios for PortfoliosMock {
    async fn load_portfolio(&self, id: &str) -> PortResult<Option<Portfolio>> {
        Ok(self
            .0
            .lock()
            .expect("test mutex must be usable")
            .get(id)
            .cloned())
    }
}

struct TradingCalendarStub;

impl ForConsultingTradingCalendar for TradingCalendarStub {
    fn is_regular_session(&self, _instant: DateTime<Utc>) -> PortResult<bool> {
        Ok(true)
    }

    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }

    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }

    fn session_open(&self, date: chrono::NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(13, 30, 0).expect("valid time").and_utc())
    }

    fn session_close(&self, date: chrono::NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(20, 0, 0).expect("valid time").and_utc())
    }
}

#[test]
fn every_declared_driven_port_accepts_a_test_double() {
    fn live(_: &impl ForObtainingLivePrices) {}
    fn instrument_prices(_: &impl ForObtainingInstrumentPrices) {}
    fn market_history(_: &impl ForObtainingMarketHistory) {}
    fn chains(_: &impl ForObtainingOptionChains) {}
    fn indices(_: &impl ForObtainingVolatilityIndices) {}
    fn load_indices(_: &impl ForLoadingIndexHistory) {}
    fn curves(_: &impl ForObtainingYieldCurves) {}
    fn load_curves(_: &impl ForLoadingYieldCurves) {}
    fn load_portfolios(_: &impl ForLoadingPortfolios) {}
    fn portfolios(_: &impl ForStoringPortfolios) {}
    fn calendar(_: &impl ForConsultingTradingCalendar) {}

    live(&LivePricesMock);
    instrument_prices(&InstrumentPricesMock);
    market_history(&MarketHistoryProviderMock);
    chains(&OptionChainsMock);
    indices(&VolatilityIndicesMock);
    load_indices(&IndexHistoryStoreMock);
    curves(&YieldCurvesMock);
    load_curves(&StoredYieldCurvesMock);
    let portfolio_store = PortfoliosMock::default();
    load_portfolios(&portfolio_store);
    portfolios(&portfolio_store);
    calendar(&TradingCalendarStub);
}
