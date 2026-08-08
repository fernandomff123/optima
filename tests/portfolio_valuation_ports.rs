use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::portfolio_valuation::{PortfolioValuationApplication, PricingCollaborators},
    domain::{
        live_price::LivePrice,
        market_history::MarketHistory,
        options::{ContratoOpcao, Snapshot},
        portfolio::{
            Currency, Instrument, Money, Portfolio, PortfolioEvent, Trade, TradeSide, decimal,
        },
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_portfolios::ForLoadingPortfolios,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_option_chains::ForObtainingOptionChains,
    },
    driving_ports::for_viewing_portfolio_positions::ForViewingPortfolioPositions,
};

struct PortfolioLoaderMock(Portfolio);

#[async_trait]
impl ForLoadingPortfolios for PortfolioLoaderMock {
    async fn load_portfolio(&self, _id: &str) -> PortResult<Option<Portfolio>> {
        Ok(Some(self.0.clone()))
    }
}

struct OpenCalendar;

impl ForConsultingTradingCalendar for OpenCalendar {
    fn is_regular_session(&self, _instant: DateTime<Utc>) -> PortResult<bool> {
        Ok(true)
    }
    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }
    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }
    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(13, 30, 0).unwrap().and_utc())
    }
    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(20, 0, 0).unwrap().and_utc())
    }
}

struct UnusedLivePrices;
#[async_trait]
impl ForObtainingLivePrices for UnusedLivePrices {
    async fn obtain_live_price(&self, _ticker: &str) -> PortResult<LivePrice> {
        Err(PortError::Unavailable("not expected".into()))
    }
}

struct UnusedHistory;
#[async_trait]
impl ForLoadingMarketHistory for UnusedHistory {
    async fn load_market_history(&self, _ticker: &str) -> PortResult<MarketHistory> {
        Err(PortError::Unavailable("not expected".into()))
    }
}

struct LiveOptionsMock;
#[async_trait]
impl ForObtainingOptionChains for LiveOptionsMock {
    async fn obtain_option_chain(&self, ticker: &str) -> PortResult<Snapshot> {
        Ok(Snapshot {
            ticker: ticker.to_string(),
            timestamp_utc: Utc.with_ymd_and_hms(2026, 8, 3, 15, 0, 0).unwrap(),
            contratos: vec![ContratoOpcao {
                occ_symbol: "SPXW  260821C05000000".into(),
                option_type: hexagonal_backend::hexagon::domain::options::OptionType::Call,
                strike: 5000.0,
                expiration: NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
                bid: 2.0,
                ask: 3.0,
                mid: 2.5,
                spread: 1.0,
                volume: 1.0,
                open_interest: 1.0,
                delta: 0.5,
                gamma: 0.1,
                vega: 0.1,
                theta: -0.1,
                rho: 0.1,
                theo: 2.5,
                implied_volatility: Some(0.2),
            }],
            chains: Vec::new(),
        })
    }
}

struct UnusedStoredOptions;
#[async_trait]
impl ForLoadingOptionChains for UnusedStoredOptions {
    async fn load_option_chain(&self, _ticker: &str) -> PortResult<Option<Snapshot>> {
        Ok(None)
    }
}

#[tokio::test]
async fn values_option_positions_through_specialized_mocked_actors() {
    let mut portfolio = Portfolio::new("main", "Principal", Currency::eur()).unwrap();
    let trade = Trade::new(
        "trade-1",
        Instrument::Option {
            occ_symbol: "SPXW  260821C05000000".to_string(),
        },
        TradeSide::Buy,
        Utc.with_ymd_and_hms(2026, 8, 3, 14, 0, 0).unwrap(),
        decimal("2").unwrap(),
        Money::new(decimal("2").unwrap(), Currency::new("USD").unwrap()),
    )
    .unwrap();
    portfolio.record(PortfolioEvent::Trade(trade)).unwrap();
    let pricing = PricingCollaborators::new(
        OpenCalendar,
        UnusedLivePrices,
        UnusedHistory,
        LiveOptionsMock,
        UnusedStoredOptions,
    );
    let application = PortfolioValuationApplication::new(PortfolioLoaderMock(portfolio), pricing);

    let positions = application.valued_positions("main").await.unwrap();

    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].market_value, Some(500.0));
    assert_eq!(
        positions[0].market_price.as_ref().unwrap().source,
        "live option chain"
    );
}
