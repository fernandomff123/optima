use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use hexagonal_backend::hexagon::{
    PortResult,
    application::options::OptionsApplication,
    domain::{
        options::{ContratoOpcao, OptionChain, OptionType, Snapshot},
        treasury::YieldCurve,
        volatility::TermStructure,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_reference_prices::ForLoadingReferencePrices,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
        for_loading_yield_curves::ForLoadingYieldCurves,
    },
    driving_ports::for_analyzing_options::{ForAnalyzingOptions, GreeksRequest},
};

#[derive(Clone)]
struct OptionDataMock {
    snapshot: Snapshot,
    term_structure: Option<TermStructure>,
    yield_curve: Option<YieldCurve>,
    reference_price: f64,
}

#[async_trait]
impl ForLoadingOptionChains for OptionDataMock {
    async fn load_option_chain(&self, _ticker: &str) -> PortResult<Option<Snapshot>> {
        Ok(Some(self.snapshot.clone()))
    }
}

#[async_trait]
impl ForLoadingVolatilityTermStructures for OptionDataMock {
    async fn load_term_structure(&self, _ticker: &str) -> PortResult<Option<TermStructure>> {
        Ok(self.term_structure.clone())
    }

    async fn load_term_structure_at_or_before(
        &self,
        _ticker: &str,
        _instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>> {
        Ok(self.term_structure.clone())
    }

    async fn load_constant_maturity_volatility_history(
        &self,
        _ticker: &str,
        _target_days: f64,
    ) -> PortResult<
        Vec<hexagonal_backend::hexagon::domain::volatility::ConstantMaturityVolatilityPoint>,
    > {
        Ok(Vec::new())
    }
}

#[async_trait]
impl ForLoadingReferencePrices for OptionDataMock {
    async fn load_reference_price(&self, _ticker: &str) -> PortResult<Option<f64>> {
        Ok(Some(self.reference_price))
    }
}

#[async_trait]
impl ForLoadingYieldCurves for OptionDataMock {
    async fn load_yield_curve(&self, _on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        Ok(self.yield_curve.clone())
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

    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(13, 30, 0).expect("valid time").and_utc())
    }

    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(20, 0, 0).expect("valid time").and_utc())
    }
}

fn contract(symbol: &str, option_type: OptionType, strike: f64) -> ContratoOpcao {
    ContratoOpcao {
        occ_symbol: symbol.into(),
        option_type,
        strike,
        expiration: NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
        bid: 1.0,
        ask: 1.2,
        mid: 1.1,
        spread: 0.2,
        volume: 10.0,
        open_interest: 100.0,
        delta: 0.4,
        gamma: 0.02,
        vega: 0.1,
        theta: -0.03,
        rho: 0.01,
        theo: 1.1,
        implied_volatility: Some(0.25),
        contract_specification: None,
    }
}

fn app_with_stored_term(
    stored: bool,
) -> OptionsApplication<OptionDataMock, OptionDataMock, OptionDataMock, TradingCalendarStub> {
    let contracts = vec![
        contract("TEST-PUT", OptionType::Put, 95.0),
        contract("TEST-CALL", OptionType::Call, 105.0),
    ];
    let snapshot_time = Utc.with_ymd_and_hms(2026, 8, 3, 21, 0, 0).unwrap();
    let data = OptionDataMock {
        snapshot: Snapshot {
            ticker: "TEST".into(),
            timestamp_utc: snapshot_time,
            contratos: contracts.clone(),
            chains: vec![OptionChain {
                root: "TEST".into(),
                contratos: contracts,
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: Default::default(),
        },
        term_structure: stored.then_some(TermStructure {
            ticker: "TEST".into(),
            snapshot_timestamp: snapshot_time,
            treasury_date: NaiveDate::from_ymd_opt(2026, 8, 3).unwrap(),
            points: Vec::new(),
        }),
        yield_curve: None,
        reference_price: 100.0,
    };
    OptionsApplication::new(data.clone(), data.clone(), data, TradingCalendarStub)
}

fn app() -> OptionsApplication<OptionDataMock, OptionDataMock, OptionDataMock, TradingCalendarStub>
{
    app_with_stored_term(true)
}

#[tokio::test]
async fn missing_stored_term_structure_uses_option_analysis_inputs() {
    let app = app_with_stored_term(false);

    let error = app.term_structure("TEST").await.unwrap_err();

    assert!(matches!(
        error,
        hexagonal_backend::hexagon::PortError::NotFound(_)
    ));
    assert!(error.to_string().contains("yield curve"));
}

#[tokio::test]
async fn one_driving_port_exposes_the_complete_options_conversation() {
    let app = app();
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();

    assert_eq!(app.option_chain("test").await.unwrap().contratos.len(), 2);
    assert_eq!(app.term_structure("TEST").await.unwrap().ticker, "TEST");
    assert_eq!(
        app.volatility_surface("TEST").await.unwrap().points.len(),
        2
    );
    assert_eq!(
        app.volatility_skew("TEST", expiration)
            .await
            .unwrap()
            .points
            .len(),
        2
    );
    assert_eq!(
        app.greeks(GreeksRequest {
            ticker: "TEST".into(),
            occ_symbol: "TEST-CALL".into(),
        })
        .await
        .unwrap()
        .delta,
        0.4
    );
}
