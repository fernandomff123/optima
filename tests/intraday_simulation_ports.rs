use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::intraday_simulation::IntradaySimulationApplication,
    domain::{
        live_price::LivePrice,
        options::{ContratoOpcao, OptionChain, OptionType, Snapshot},
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_option_chains::ForObtainingOptionChains,
    },
    driving_ports::{
        for_preparing_intraday_simulations::ForPreparingIntradaySimulations,
        for_viewing_intraday_options::ForViewingIntradayOptions,
    },
};

struct OptionChainsMock(Arc<AtomicUsize>);

#[async_trait]
impl ForObtainingOptionChains for OptionChainsMock {
    async fn obtain_option_chain(&self, ticker: &str) -> PortResult<Snapshot> {
        self.0.fetch_add(1, Ordering::Relaxed);
        let expiration = Utc::now().date_naive() + chrono::Duration::days(30);
        let contract = ContratoOpcao {
            occ_symbol: "TEST-CALL".to_string(),
            option_type: OptionType::Call,
            strike: 5_300.0,
            expiration,
            bid: 10.0,
            ask: 11.0,
            mid: 10.5,
            spread: 1.0,
            volume: 100.0,
            open_interest: Some(1_000.0),
            delta: 0.4,
            gamma: Some(0.01),
            vega: 0.1,
            theta: -0.02,
            rho: 0.01,
            theo: 10.5,
            implied_volatility: Some(0.2),
            contract_specification: None,
        };
        Ok(Snapshot {
            ticker: ticker.to_string(),
            timestamp_utc: Utc::now(),
            contratos: vec![contract.clone()],
            chains: vec![OptionChain {
                root: ticker.to_string(),
                contratos: vec![contract],
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: Default::default(),
        })
    }
}

struct LivePricesMock;

#[async_trait]
impl ForObtainingLivePrices for LivePricesMock {
    async fn obtain_live_price(&self, ticker: &str) -> PortResult<LivePrice> {
        Ok(LivePrice {
            ticker: ticker.to_string(),
            price: 5_250.0,
            market_time: 0,
            currency: "USD".to_string(),
            exchange: "TEST".to_string(),
            regular_session: true,
            change: 0.0,
            change_percent: 0.0,
            day_volume: 0,
        })
    }
}

struct CalendarStub(bool);

impl ForConsultingTradingCalendar for CalendarStub {
    fn is_regular_session(&self, _instant: DateTime<Utc>) -> PortResult<bool> {
        Ok(self.0)
    }

    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }

    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }

    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(13, 30, 0).expect("valid time").and_utc())
    }

    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(20, 0, 0).expect("valid time").and_utc())
    }
}

#[tokio::test]
async fn obtains_intraday_inputs_through_mocked_driven_ports() {
    let application = IntradaySimulationApplication::new(
        OptionChainsMock(Arc::new(AtomicUsize::new(0))),
        LivePricesMock,
        CalendarStub(true),
    );

    let market = application.intraday_market(" spx ").await.unwrap();

    assert_eq!(market.snapshot.ticker, "SPX");
    assert_eq!(market.spot, 5_250.0);

    let options_market = application.intraday_options("SPX").await.unwrap();
    assert_eq!(options_market.market.spot, 5_250.0);
    let surface = options_market
        .volatility_surface
        .expect("application must calculate the intraday surface");
    assert_eq!(surface.reference_price, 5_250.0);
    assert_eq!(surface.points.len(), 1);
}

#[tokio::test]
async fn closed_session_stops_before_obtaining_external_data() {
    let calls = Arc::new(AtomicUsize::new(0));
    let chains = OptionChainsMock(Arc::clone(&calls));
    let application =
        IntradaySimulationApplication::new(chains, LivePricesMock, CalendarStub(false));

    let error = application.intraday_market("SPX").await.unwrap_err();

    assert!(matches!(error, PortError::Conflict(_)));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}
