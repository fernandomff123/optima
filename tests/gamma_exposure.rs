use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use hexagonal_backend::driven_adapters::cboe::{CboeResponse, response_to_snapshot_collected_at};
use hexagonal_backend::hexagon::domain::treasury::YieldCurve;
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::gamma_exposure::GammaExposureApplication,
    domain::{
        gamma_exposure::{
            ExclusionReason, ModeledExpirationInput, ModeledGammaExposurePoint, SnapshotOrigin,
            calculate, modeled_profile, zero_crossings,
        },
        options::{
            ContratoOpcao, OptionChain, OptionContractSpecification, OptionIngestionDiagnostics,
            OptionType, Snapshot, StoredOptionChainSnapshot, UnderlyingPriceObservation,
        },
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_resolving_option_contract_specifications::{
            ForResolvingOptionContractSpecifications, OptionContractIdentity,
            OptionContractSpecificationResolution,
        },
    },
    driving_ports::for_viewing_gamma_exposure::{ForViewingGammaExposure, GammaExposureRequest},
};

fn contract(
    symbol: &str,
    option_type: OptionType,
    strike: f64,
    expiration: NaiveDate,
) -> ContratoOpcao {
    ContratoOpcao {
        occ_symbol: symbol.to_string(),
        option_type,
        strike,
        expiration,
        bid: 1.0,
        ask: 1.2,
        mid: 1.1,
        spread: 0.2,
        volume: 0.0,
        open_interest: Some(10.0),
        delta: 0.0,
        gamma: Some(0.02),
        vega: 0.0,
        theta: 0.0,
        rho: 0.0,
        theo: 0.0,
        implied_volatility: Some(0.2),
        contract_specification: Some(OptionContractSpecification {
            root: "SPX".to_string(),
            contract_multiplier: 100.0,
            currency: "USD".to_string(),
            source_reference: "factual test specification".to_string(),
            catalog_reviewed_at: None,
            effective_from: None,
        }),
    }
}

fn snapshot(contracts: Vec<ContratoOpcao>) -> Snapshot {
    let timestamp = Utc.with_ymd_and_hms(2026, 8, 21, 20, 0, 0).unwrap();
    Snapshot {
        ticker: "SPX".to_string(),
        timestamp_utc: timestamp,
        contratos: contracts.clone(),
        chains: vec![OptionChain {
            root: "SPX".into(),
            contratos: contracts,
        }],
        underlying_price: Some(
            UnderlyingPriceObservation::new(
                100.0,
                Some(timestamp),
                Some("USD".to_string()),
                "test",
            )
            .unwrap(),
        ),
        collected_at: Some(timestamp),
        provider_timestamp: None,
        ingestion_diagnostics: OptionIngestionDiagnostics::default(),
    }
}

#[derive(Clone)]
struct YieldCurves;

#[async_trait]
impl ForLoadingYieldCurves for YieldCurves {
    async fn load_yield_curve(&self, _: NaiveDate) -> PortResult<Option<YieldCurve>> {
        Ok(Some(YieldCurve {
            date: NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
            m1: Some(0.04),
            m2: Some(0.041),
            m3: Some(0.042),
            m6: Some(0.043),
            y1: Some(0.044),
            y2: None,
            y3: None,
            y5: None,
            y7: None,
            y10: None,
            y20: None,
            y30: None,
        }))
    }
}

#[derive(Clone)]
struct FailingYieldCurves {
    calls: Arc<AtomicUsize>,
    result: PortResult<Option<YieldCurve>>,
}

#[async_trait]
impl ForLoadingYieldCurves for FailingYieldCurves {
    async fn load_yield_curve(&self, _: NaiveDate) -> PortResult<Option<YieldCurve>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
}

fn request(ticker: &str) -> GammaExposureRequest {
    GammaExposureRequest {
        ticker: ticker.into(),
        range_percent: 20.0,
        points: 81,
        valuation_time: Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 0).unwrap(),
    }
}

fn specification(root: &str, multiplier: f64, currency: &str) -> OptionContractSpecification {
    OptionContractSpecification::new(root, multiplier, currency, "test catalog", None, None)
        .unwrap()
}

fn provider_snapshot(chains: Vec<OptionChain>) -> Snapshot {
    let mut value = snapshot(Vec::new());
    value.underlying_price.as_mut().unwrap().currency = None;
    value.contratos = chains
        .iter()
        .flat_map(|chain| chain.contratos.iter().cloned())
        .collect();
    value.chains = chains;
    value
}

fn stored_snapshot(snapshot: Snapshot) -> StoredOptionChainSnapshot {
    StoredOptionChainSnapshot {
        session_date: snapshot.timestamp_utc.date_naive(),
        snapshot,
    }
}

#[test]
fn known_formula_signs_and_aggregations_are_deterministic() {
    let first = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let second = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
    let contracts = vec![
        contract("CALL-1", OptionType::Call, 100.0, first),
        contract("PUT-1", OptionType::Put, 100.0, first),
        contract("CALL-2", OptionType::Call, 110.0, second),
    ];
    let result = calculate(&snapshot(contracts), SnapshotOrigin::Intraday);

    // 0.02 × 10 × 100 × 100² × 0.01 = 2_000 per contract.
    assert_eq!(result.calls_gex, 4_000.0);
    assert_eq!(result.puts_gex, -2_000.0);
    assert_eq!(result.net_gex, 2_000.0);
    assert_eq!(result.by_strike.len(), 2);
    assert_eq!(result.by_strike[0].net_gex, 0.0);
    assert_eq!(result.by_strike[1].net_gex, 2_000.0);
    assert_eq!(result.by_expiration.len(), 2);
    assert_eq!(result.by_expiration[0].net_gex, 0.0);
    assert_eq!(result.by_expiration[1].net_gex, 2_000.0);
    assert!(result.sign_convention.contains("Analytical assumption"));
    assert_eq!(result.currency.as_deref(), Some("USD"));
}

#[test]
fn published_zero_values_are_included_and_partial_results_survive_exclusions() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let mut zero = contract("ZERO", OptionType::Call, 100.0, expiration);
    zero.gamma = Some(0.0);
    zero.open_interest = Some(0.0);
    let mut missing = contract("MISSING", OptionType::Put, 100.0, expiration);
    missing.gamma = None;
    let result = calculate(&snapshot(vec![zero, missing]), SnapshotOrigin::EndOfDay);

    assert_eq!(result.calls_gex, 0.0);
    assert_eq!(result.diagnostics.included_contracts, 1);
    assert_eq!(result.diagnostics.excluded_contracts, 1);
    assert_eq!(
        result.diagnostics.excluded_by_reason[&ExclusionReason::MissingGamma],
        1
    );
}

fn modeled_snapshot() -> Snapshot {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let mut call = contract("CALL", OptionType::Call, 100.0, expiration);
    call.mid = 5.0;
    let mut put = contract("PUT", OptionType::Put, 100.0, expiration);
    put.mid = 4.0;
    snapshot(vec![call, put])
}

fn modeled_inputs() -> BTreeMap<(String, NaiveDate), ModeledExpirationInput> {
    BTreeMap::from([(
        ("SPX".into(), NaiveDate::from_ymd_opt(2026, 9, 18).unwrap()),
        ModeledExpirationInput {
            time_to_expiration: 28.0 / 365.0,
            interest_rate: 0.04,
            dividend_yield: 0.02,
        },
    )])
}

#[test]
fn modeled_profile_reuses_black_scholes_on_a_centered_bounded_grid() {
    let valuation = Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 0).unwrap();
    for points in [21, 81, 201] {
        let profile = modeled_profile(
            &modeled_snapshot(),
            valuation,
            20.0,
            points,
            &modeled_inputs(),
            &BTreeSet::new(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(profile.profile.len(), points);
        assert_eq!(profile.profile[points / 2].spot, 100.0);
        assert!(profile.profile.iter().all(|point| {
            point.call_gex >= 0.0
                && point.put_gex <= 0.0
                && point.call_gex.is_finite()
                && point.put_gex.is_finite()
                && point.net_gex.is_finite()
        }));
        assert!(profile.sticky_strike_assumption.contains("sticky-strike"));
    }
    for (range, points) in [(4.9, 81), (50.1, 81), (20.0, 20), (20.0, 202), (20.0, 80)] {
        assert!(
            modeled_profile(
                &modeled_snapshot(),
                valuation,
                range,
                points,
                &modeled_inputs(),
                &BTreeSet::new(),
            )
            .is_err()
        );
    }
}

#[test]
fn modeled_profile_uses_factual_settlement_time_for_same_day_and_prior_expirations() {
    let valuation = Utc.with_ymd_and_hms(2026, 9, 18, 15, 0, 0).unwrap();
    let same_day = valuation.date_naive();
    let future = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
    let prior = NaiveDate::from_ymd_opt(2026, 9, 17).unwrap();
    let make_snapshot = |expiration| {
        let mut value = snapshot(vec![contract(
            "SPXW-CONTRACT",
            OptionType::Call,
            100.0,
            expiration,
        )]);
        value.chains[0].root = "SPXW".into();
        value
    };
    let input = ModeledExpirationInput {
        time_to_expiration: 5.0 / 24.0 / 365.0,
        interest_rate: 0.04,
        dividend_yield: 0.02,
    };

    let before_pm_close = modeled_profile(
        &make_snapshot(same_day),
        valuation,
        20.0,
        21,
        &BTreeMap::from([(("SPXW".into(), same_day), input)]),
        &BTreeSet::new(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(before_pm_close.included_contracts, 1);

    for expiration in [same_day, prior] {
        let mut value = make_snapshot(expiration);
        let control = contract("FUTURE", OptionType::Call, 105.0, future);
        value.chains[0].contratos.push(control.clone());
        value.contratos.push(control);
        let result = modeled_profile(
            &value,
            valuation,
            20.0,
            21,
            &BTreeMap::from([(("SPXW".into(), future), input)]),
            &BTreeSet::from([("SPXW".into(), expiration)]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.included_contracts, 1);
        assert_eq!(
            result.excluded_by_reason[&ExclusionReason::ExpiredContract],
            1
        );
    }
}

#[test]
fn zero_dte_without_forward_before_settlement_is_missing_carry_not_expired() {
    let valuation = Utc.with_ymd_and_hms(2026, 9, 18, 15, 0, 0).unwrap();
    let same_day = valuation.date_naive();
    let future = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
    let zero_dte = contract("ZERO-DTE", OptionType::Call, 100.0, same_day);
    let calculable = contract("CALCULABLE", OptionType::Call, 105.0, future);
    let mut value = snapshot(vec![zero_dte.clone(), calculable.clone()]);
    value.chains = vec![OptionChain {
        root: "SPXW".into(),
        contratos: vec![zero_dte, calculable],
    }];
    let inputs = BTreeMap::from([(
        ("SPXW".into(), future),
        ModeledExpirationInput {
            time_to_expiration: 3.0 / 365.0,
            interest_rate: 0.04,
            dividend_yield: 0.02,
        },
    )]);

    let result = modeled_profile(&value, valuation, 20.0, 21, &inputs, &BTreeSet::new())
        .unwrap()
        .unwrap();
    assert_eq!(result.included_contracts, 1);
    assert_eq!(
        result.excluded_by_reason[&ExclusionReason::MissingForwardCarry],
        1
    );
    assert!(
        !result
            .excluded_by_reason
            .contains_key(&ExclusionReason::ExpiredContract)
    );
}

#[test]
fn modeled_profile_diagnostics_distinguish_missing_and_invalid_inputs() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let valuation = Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 0).unwrap();
    let mut contracts = vec![contract("VALID", OptionType::Call, 100.0, expiration)];
    let mut missing_oi = contract("MISSING-OI", OptionType::Call, 101.0, expiration);
    missing_oi.open_interest = None;
    contracts.push(missing_oi);
    let mut invalid_oi = contract("INVALID-OI", OptionType::Call, 102.0, expiration);
    invalid_oi.open_interest = Some(f64::NAN);
    contracts.push(invalid_oi);
    let mut missing_multiplier = contract("MISSING-MULT", OptionType::Call, 103.0, expiration);
    missing_multiplier.contract_specification = None;
    contracts.push(missing_multiplier);
    let mut invalid_multiplier = contract("INVALID-MULT", OptionType::Call, 104.0, expiration);
    invalid_multiplier
        .contract_specification
        .as_mut()
        .unwrap()
        .contract_multiplier = 0.0;
    contracts.push(invalid_multiplier);
    let mut missing_iv = contract("MISSING-IV", OptionType::Call, 105.0, expiration);
    missing_iv.implied_volatility = None;
    contracts.push(missing_iv);
    let mut invalid_iv = contract("INVALID-IV", OptionType::Call, 106.0, expiration);
    invalid_iv.implied_volatility = Some(f64::INFINITY);
    contracts.push(invalid_iv);

    let result = modeled_profile(
        &snapshot(contracts),
        valuation,
        20.0,
        21,
        &modeled_inputs(),
        &BTreeSet::new(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(result.included_contracts, 1);
    for reason in [
        ExclusionReason::MissingOpenInterest,
        ExclusionReason::InvalidOpenInterest,
        ExclusionReason::MissingMultiplier,
        ExclusionReason::InvalidMultiplier,
        ExclusionReason::MissingImpliedVolatility,
        ExclusionReason::InvalidImpliedVolatility,
    ] {
        assert_eq!(result.excluded_by_reason[&reason], 1);
    }
}

#[test]
fn modeled_center_is_independent_from_provider_gamma_and_exclusions_are_partial() {
    let valuation = Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 0).unwrap();
    let mut value = modeled_snapshot();
    value.contratos[0].gamma = Some(0.000_001);
    value.chains[0].contratos[0].gamma = Some(0.000_001);
    let current = calculate(&value, SnapshotOrigin::Intraday);
    let modeled = modeled_profile(
        &value,
        valuation,
        20.0,
        81,
        &modeled_inputs(),
        &BTreeSet::new(),
    )
    .unwrap()
    .unwrap();
    assert_ne!(current.net_gex, modeled.profile[40].net_gex);

    value.chains[0].contratos[1].implied_volatility = None;
    let partial = modeled_profile(
        &value,
        valuation,
        20.0,
        81,
        &modeled_inputs(),
        &BTreeSet::new(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(partial.included_contracts, 1);
    assert_eq!(partial.excluded_contracts, 1);
    assert_eq!(
        partial.excluded_by_reason[&ExclusionReason::MissingImpliedVolatility],
        1
    );
}

#[test]
fn modeled_profile_rejects_missing_carry_and_numeric_overflow_without_non_finite_output() {
    let valuation = Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 0).unwrap();
    assert!(
        modeled_profile(
            &modeled_snapshot(),
            valuation,
            20.0,
            81,
            &BTreeMap::new(),
            &BTreeSet::new(),
        )
        .unwrap()
        .is_none()
    );
    let mut extreme = modeled_snapshot();
    for contract in &mut extreme.chains[0].contratos {
        contract.open_interest = Some(f64::MAX);
    }
    assert!(
        modeled_profile(
            &extreme,
            valuation,
            20.0,
            81,
            &modeled_inputs(),
            &BTreeSet::new(),
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn zero_crossings_reports_none_one_and_multiple_with_linear_interpolation() {
    let point = |spot: f64, net_gex: f64| ModeledGammaExposurePoint {
        spot,
        call_gex: net_gex.max(0.0),
        put_gex: net_gex.min(0.0),
        net_gex,
    };
    assert!(zero_crossings(&[point(90.0, 1.0), point(100.0, 2.0)]).is_empty());
    assert_eq!(
        zero_crossings(&[point(90.0, -1.0), point(100.0, 1.0)]),
        vec![95.0]
    );
    assert_eq!(
        zero_crossings(&[point(80.0, -1.0), point(90.0, 1.0), point(100.0, -1.0),]),
        vec![85.0, 95.0]
    );
}

#[test]
fn exclusions_distinguish_every_missing_and_invalid_input() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let mut missing_gamma = contract("MG", OptionType::Call, 1.0, expiration);
    missing_gamma.gamma = None;
    let mut invalid_gamma = contract("IG", OptionType::Call, 2.0, expiration);
    invalid_gamma.gamma = Some(f64::NAN);
    let mut missing_oi = contract("MO", OptionType::Call, 3.0, expiration);
    missing_oi.open_interest = None;
    let mut invalid_oi = contract("IO", OptionType::Call, 4.0, expiration);
    invalid_oi.open_interest = Some(-1.0);
    let mut missing_multiplier = contract("MM", OptionType::Call, 5.0, expiration);
    missing_multiplier.contract_specification = None;
    let mut invalid_multiplier = contract("IM", OptionType::Call, 6.0, expiration);
    invalid_multiplier
        .contract_specification
        .as_mut()
        .unwrap()
        .contract_multiplier = 0.0;
    let contracts = vec![
        missing_gamma,
        invalid_gamma,
        missing_oi,
        invalid_oi,
        missing_multiplier,
        invalid_multiplier,
    ];
    let result = calculate(&snapshot(contracts.clone()), SnapshotOrigin::Intraday);
    for reason in [
        ExclusionReason::MissingGamma,
        ExclusionReason::InvalidGamma,
        ExclusionReason::MissingOpenInterest,
        ExclusionReason::InvalidOpenInterest,
        ExclusionReason::MissingMultiplier,
        ExclusionReason::InvalidMultiplier,
    ] {
        assert_eq!(result.diagnostics.excluded_by_reason[&reason], 1);
    }

    let mut missing_spot = snapshot(contracts.clone());
    missing_spot.underlying_price = None;
    let missing = calculate(&missing_spot, SnapshotOrigin::Intraday);
    assert_eq!(
        missing.diagnostics.excluded_by_reason[&ExclusionReason::MissingSpot],
        6
    );
    let mut invalid_spot = snapshot(contracts);
    invalid_spot.underlying_price.as_mut().unwrap().value = f64::INFINITY;
    let invalid = calculate(&invalid_spot, SnapshotOrigin::Intraday);
    assert_eq!(
        invalid.diagnostics.excluded_by_reason[&ExclusionReason::InvalidSpot],
        6
    );
}

#[derive(Clone)]
struct Calendar(bool);

impl ForConsultingTradingCalendar for Calendar {
    fn is_regular_session(&self, _: DateTime<Utc>) -> PortResult<bool> {
        Ok(self.0)
    }
    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }
    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }
    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(14, 30, 0).unwrap().and_utc())
    }
    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        Ok(date.and_hms_opt(21, 0, 0).unwrap().and_utc())
    }
}

#[derive(Clone)]
struct EarlyCloseCalendar {
    requested_closes: Arc<Mutex<Vec<NaiveDate>>>,
}

#[derive(Clone)]
struct SettlementCalendar {
    opens: Arc<Mutex<Vec<NaiveDate>>>,
    closes: Arc<Mutex<Vec<NaiveDate>>>,
}

impl ForConsultingTradingCalendar for SettlementCalendar {
    fn is_regular_session(&self, _: DateTime<Utc>) -> PortResult<bool> {
        Ok(true)
    }
    fn next_session_transition(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }
    fn latest_session_close_before(&self, instant: DateTime<Utc>) -> PortResult<DateTime<Utc>> {
        Ok(instant)
    }
    fn session_open(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        self.opens.lock().unwrap().push(date);
        Ok(date.and_hms_opt(14, 30, 0).unwrap().and_utc())
    }
    fn session_close(&self, date: NaiveDate) -> PortResult<DateTime<Utc>> {
        self.closes.lock().unwrap().push(date);
        Ok(date.and_hms_opt(21, 0, 0).unwrap().and_utc())
    }
}

impl ForConsultingTradingCalendar for EarlyCloseCalendar {
    fn is_regular_session(&self, _: DateTime<Utc>) -> PortResult<bool> {
        Ok(false)
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
        self.requested_closes.lock().unwrap().push(date);
        let hour = if date == NaiveDate::from_ymd_opt(2026, 8, 21).unwrap() {
            18
        } else {
            21
        };
        Ok(date.and_hms_opt(hour, 0, 0).unwrap().and_utc())
    }
}

#[derive(Clone)]
struct Intraday {
    calls: Arc<AtomicUsize>,
    result: PortResult<Snapshot>,
}

#[async_trait]
impl ForObtainingOptionChains for Intraday {
    async fn obtain_option_chain(&self, _: &str) -> PortResult<Snapshot> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
}

#[derive(Clone)]
struct Stored {
    calls: Arc<AtomicUsize>,
    result: PortResult<Option<StoredOptionChainSnapshot>>,
}

#[derive(Clone)]
struct Specifications {
    calls: Arc<AtomicUsize>,
    requested: Arc<Mutex<Vec<Vec<OptionContractIdentity>>>>,
    result: PortResult<BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>>,
}

#[async_trait]
impl ForResolvingOptionContractSpecifications for Specifications {
    async fn resolve_option_contract_specifications(
        &self,
        identities: &[OptionContractIdentity],
    ) -> PortResult<BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.requested.lock().unwrap().push(identities.to_vec());
        self.result.clone()
    }
}

#[async_trait]
impl ForLoadingOptionChains for Stored {
    async fn load_option_chain(&self, _: &str) -> PortResult<Option<StoredOptionChainSnapshot>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
}

fn applications(
    regular: bool,
    intraday: PortResult<Snapshot>,
    stored: PortResult<Option<StoredOptionChainSnapshot>>,
) -> (TestApplication, ApplicationCalls) {
    let intraday_calls = Arc::new(AtomicUsize::new(0));
    let stored_calls = Arc::new(AtomicUsize::new(0));
    let resolver_calls = Arc::new(AtomicUsize::new(0));
    (
        GammaExposureApplication::new(
            Calendar(regular),
            Intraday {
                calls: intraday_calls.clone(),
                result: intraday,
            },
            Stored {
                calls: stored_calls.clone(),
                result: stored,
            },
            Specifications {
                calls: resolver_calls.clone(),
                requested: Arc::new(Mutex::new(Vec::new())),
                result: Ok(BTreeMap::new()),
            },
            YieldCurves,
        ),
        ApplicationCalls {
            provider: intraday_calls,
            storage: stored_calls,
            resolver: resolver_calls,
        },
    )
}

type TestApplication =
    GammaExposureApplication<Calendar, Intraday, Stored, Specifications, YieldCurves>;

struct ApplicationCalls {
    provider: Arc<AtomicUsize>,
    storage: Arc<AtomicUsize>,
    resolver: Arc<AtomicUsize>,
}

#[tokio::test]
async fn regular_session_uses_transient_intraday_snapshot_without_storage() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let identity = OptionContractIdentity {
        root: "SPX".into(),
        occ_symbol: "SPX   260918C00100000".into(),
    };
    let mut value = contract(&identity.occ_symbol, OptionType::Call, 100.0, expiration);
    value.contract_specification = None;
    let provider = Arc::new(AtomicUsize::new(0));
    let storage = Arc::new(AtomicUsize::new(0));
    let resolver = Arc::new(AtomicUsize::new(0));
    let app = GammaExposureApplication::new(
        Calendar(true),
        Intraday {
            calls: provider.clone(),
            result: Ok(provider_snapshot(vec![OptionChain {
                root: identity.root.clone(),
                contratos: vec![value],
            }])),
        },
        Stored {
            calls: storage.clone(),
            result: Ok(None),
        },
        Specifications {
            calls: resolver.clone(),
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Ok(BTreeMap::from([(
                identity,
                OptionContractSpecificationResolution::Found(specification("SPX", 100.0, "USD")),
            )])),
        },
        YieldCurves,
    );
    let result = app.gamma_exposure(request(" spx ")).await.unwrap();
    assert_eq!(
        result.current_exposure.snapshot_origin,
        SnapshotOrigin::Intraday
    );
    assert_eq!(provider.load(Ordering::SeqCst), 1);
    assert_eq!(storage.load(Ordering::SeqCst), 0);
    assert_eq!(resolver.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn intraday_spxw_0dte_uses_pm_close_while_spx_preserves_am_settlement() {
    let expiration = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
    let pair = |root: &str| {
        let mut call = contract(&format!("{root}-CALL"), OptionType::Call, 100.0, expiration);
        call.mid = 5.0;
        let mut put = contract(&format!("{root}-PUT"), OptionType::Put, 100.0, expiration);
        put.mid = 4.0;
        OptionChain {
            root: root.into(),
            contratos: vec![call, put],
        }
    };
    let opens = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(Vec::new()));
    let app = GammaExposureApplication::new(
        SettlementCalendar {
            opens: opens.clone(),
            closes: closes.clone(),
        },
        Intraday {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Ok(provider_snapshot(vec![pair("SPXW"), pair("SPX")])),
        },
        Stored {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Ok(None),
        },
        Specifications {
            calls: Arc::new(AtomicUsize::new(0)),
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Ok(BTreeMap::from([
                (
                    OptionContractIdentity {
                        root: "SPXW".into(),
                        occ_symbol: "SPXW-CALL".into(),
                    },
                    OptionContractSpecificationResolution::Found(specification(
                        "SPXW", 100.0, "USD",
                    )),
                ),
                (
                    OptionContractIdentity {
                        root: "SPXW".into(),
                        occ_symbol: "SPXW-PUT".into(),
                    },
                    OptionContractSpecificationResolution::Found(specification(
                        "SPXW", 100.0, "USD",
                    )),
                ),
                (
                    OptionContractIdentity {
                        root: "SPX".into(),
                        occ_symbol: "SPX-CALL".into(),
                    },
                    OptionContractSpecificationResolution::Found(specification(
                        "SPX", 100.0, "USD",
                    )),
                ),
                (
                    OptionContractIdentity {
                        root: "SPX".into(),
                        occ_symbol: "SPX-PUT".into(),
                    },
                    OptionContractSpecificationResolution::Found(specification(
                        "SPX", 100.0, "USD",
                    )),
                ),
            ])),
        },
        YieldCurves,
    );
    let result = app.gamma_exposure(request("SPX")).await.unwrap();
    let profile = result.modeled_profile.unwrap();
    assert_eq!(profile.included_contracts, 2);
    assert_eq!(profile.excluded_contracts, 2);
    assert_eq!(
        profile.excluded_by_reason[&ExclusionReason::ExpiredContract],
        2
    );
    assert_eq!(opens.lock().unwrap().as_slice(), &[expiration]);
    assert_eq!(closes.lock().unwrap().as_slice(), &[expiration]);
}

#[tokio::test]
async fn outside_session_uses_eod_without_provider_and_reports_absence() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let eod = snapshot(vec![contract("P", OptionType::Put, 100.0, expiration)]);
    let (app, calls) = applications(
        false,
        Err(PortError::Unavailable("must not call".into())),
        Ok(Some(stored_snapshot(eod))),
    );
    let result = app.gamma_exposure(request("SPX")).await.unwrap();
    assert_eq!(
        result.current_exposure.snapshot_origin,
        SnapshotOrigin::EndOfDay
    );
    assert!(result.modeled_profile.is_none());
    assert_eq!(calls.provider.load(Ordering::SeqCst), 0);
    assert_eq!(calls.storage.load(Ordering::SeqCst), 1);
    assert_eq!(calls.resolver.load(Ordering::SeqCst), 0);

    let (missing, calls) = applications(
        false,
        Err(PortError::Unavailable("must not call".into())),
        Ok(None),
    );
    assert!(
        matches!(missing.gamma_exposure(request("SPX")).await, Err(PortError::Unavailable(message)) if message.contains("end-of-day"))
    );
    assert_eq!(calls.provider.load(Ordering::SeqCst), 0);
    assert_eq!(calls.resolver.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn eod_profile_uses_the_loaded_sessions_official_early_close() {
    let session_date = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
    let requested_closes = Arc::new(Mutex::new(Vec::new()));
    let app = GammaExposureApplication::new(
        EarlyCloseCalendar {
            requested_closes: requested_closes.clone(),
        },
        Intraday {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Err(PortError::Unavailable("must not call".into())),
        },
        Stored {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Ok(Some(StoredOptionChainSnapshot {
                snapshot: modeled_snapshot(),
                session_date,
            })),
        },
        Specifications {
            calls: Arc::new(AtomicUsize::new(0)),
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Ok(BTreeMap::new()),
        },
        YieldCurves,
    );
    let result = app.gamma_exposure(request("SPX")).await.unwrap();
    assert_eq!(
        result.modeled_profile.unwrap().valuation_time,
        session_date.and_hms_opt(18, 0, 0).unwrap().and_utc()
    );
    assert!(requested_closes.lock().unwrap().contains(&session_date));
}

#[tokio::test]
async fn missing_or_failed_yield_curve_preserves_current_exposure() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    for curve_result in [
        Ok(None),
        Err(PortError::Unavailable("curve loader failed".into())),
    ] {
        let curve_calls = Arc::new(AtomicUsize::new(0));
        let app = GammaExposureApplication::new(
            Calendar(false),
            Intraday {
                calls: Arc::new(AtomicUsize::new(0)),
                result: Err(PortError::Unavailable("must not call".into())),
            },
            Stored {
                calls: Arc::new(AtomicUsize::new(0)),
                result: Ok(Some(stored_snapshot(snapshot(vec![contract(
                    "CURRENT",
                    OptionType::Call,
                    100.0,
                    expiration,
                )])))),
            },
            Specifications {
                calls: Arc::new(AtomicUsize::new(0)),
                requested: Arc::new(Mutex::new(Vec::new())),
                result: Ok(BTreeMap::new()),
            },
            FailingYieldCurves {
                calls: curve_calls.clone(),
                result: curve_result,
            },
        );
        let result = app.gamma_exposure(request("SPX")).await.unwrap();
        assert_eq!(result.current_exposure.diagnostics.included_contracts, 1);
        assert!(result.current_exposure.calls_gex > 0.0);
        assert!(result.modeled_profile.is_none());
        assert_eq!(curve_calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn provider_errors_are_propagated_without_storage_fallback() {
    let (app, calls) = applications(
        true,
        Err(PortError::Unavailable("provider failed".into())),
        Ok(None),
    );
    assert_eq!(
        app.gamma_exposure(request("SPX")).await,
        Err(PortError::Unavailable("provider failed".into()))
    );
    assert_eq!(calls.provider.load(Ordering::SeqCst), 1);
    assert_eq!(calls.storage.load(Ordering::SeqCst), 0);
    assert_eq!(calls.resolver.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn intraday_snapshot_is_enriched_once_with_deduplicated_identities_and_partial_not_found() {
    let found = OptionContractIdentity {
        root: "SPX".into(),
        occ_symbol: "SPX   260918C00100000".into(),
    };
    let absent = OptionContractIdentity {
        root: "SPX1".into(),
        occ_symbol: "SPX1  260918P00100000".into(),
    };
    let response: CboeResponse = serde_json::from_value(serde_json::json!({
        "timestamp": "2026-08-21T15:00:00Z",
        "data": {
            "current_price": 100.0,
            "options": [
                { "option": found.occ_symbol.clone(), "bid": 1.0, "ask": 1.1, "volume": 1.0, "open_interest": 10.0, "delta": 0.5, "gamma": 0.02, "vega": 0.1, "theta": -0.1, "rho": 0.1, "theo": 1.0, "iv": 0.2 },
                { "option": found.occ_symbol.clone(), "bid": 1.0, "ask": 1.1, "volume": 1.0, "open_interest": 5.0, "delta": 0.5, "gamma": 0.02, "vega": 0.1, "theta": -0.1, "rho": 0.1, "theo": 1.0, "iv": 0.2 },
                { "option": absent.occ_symbol.clone(), "bid": 1.0, "ask": 1.1, "volume": 1.0, "open_interest": 10.0, "delta": -0.5, "gamma": 0.02, "vega": 0.1, "theta": -0.1, "rho": 0.1, "theo": 1.0, "iv": 0.2 }
            ]
        }
    })).unwrap();
    let intraday = response_to_snapshot_collected_at(
        "SPX",
        response,
        Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 1).unwrap(),
    )
    .unwrap();
    assert!(
        intraday
            .contratos
            .iter()
            .all(|contract| contract.contract_specification.is_none())
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let requested = Arc::new(Mutex::new(Vec::new()));
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let storage_calls = Arc::new(AtomicUsize::new(0));
    let app = GammaExposureApplication::new(
        Calendar(true),
        Intraday {
            calls: provider_calls,
            result: Ok(intraday),
        },
        Stored {
            calls: storage_calls.clone(),
            result: Ok(None),
        },
        Specifications {
            calls: calls.clone(),
            requested: requested.clone(),
            result: Ok(BTreeMap::from([
                (
                    found.clone(),
                    OptionContractSpecificationResolution::Found(specification(
                        "SPX", 100.0, "USD",
                    )),
                ),
                (
                    absent.clone(),
                    OptionContractSpecificationResolution::NotFound,
                ),
            ])),
        },
        YieldCurves,
    );

    let result = app.gamma_exposure(request("SPX")).await.unwrap();
    assert!(result.current_exposure.net_gex > 0.0);
    assert_eq!(result.current_exposure.diagnostics.included_contracts, 2);
    assert_eq!(result.current_exposure.diagnostics.excluded_contracts, 1);
    assert_eq!(
        result.current_exposure.diagnostics.excluded_by_reason[&ExclusionReason::MissingMultiplier],
        1
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(requested.lock().unwrap().as_slice(), &[vec![found, absent]]);
    assert_eq!(storage_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn intraday_rejects_incomplete_or_additional_resolution_identity_sets() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let requested = OptionContractIdentity {
        root: "SPX".into(),
        occ_symbol: "SPX   260918C00100000".into(),
    };
    let additional = OptionContractIdentity {
        root: "XSP".into(),
        occ_symbol: "XSP   260918C00100000".into(),
    };
    let mut value = contract(&requested.occ_symbol, OptionType::Call, 100.0, expiration);
    value.contract_specification = None;
    let snapshot = provider_snapshot(vec![OptionChain {
        root: requested.root.clone(),
        contratos: vec![value],
    }]);
    for result in [
        BTreeMap::new(),
        BTreeMap::from([
            (
                requested.clone(),
                OptionContractSpecificationResolution::Found(specification("SPX", 100.0, "USD")),
            ),
            (
                additional.clone(),
                OptionContractSpecificationResolution::NotFound,
            ),
        ]),
    ] {
        let storage_calls = Arc::new(AtomicUsize::new(0));
        let app = GammaExposureApplication::new(
            Calendar(true),
            Intraday {
                calls: Arc::new(AtomicUsize::new(0)),
                result: Ok(snapshot.clone()),
            },
            Stored {
                calls: storage_calls.clone(),
                result: Ok(None),
            },
            Specifications {
                calls: Arc::new(AtomicUsize::new(0)),
                requested: Arc::new(Mutex::new(Vec::new())),
                result: Ok(result),
            },
            YieldCurves,
        );
        assert!(
            matches!(app.gamma_exposure(request("SPX")).await, Err(PortError::Unavailable(message)) if message.contains("incompatible identity set"))
        );
        assert_eq!(storage_calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn resolver_failure_is_propagated_without_persistence() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let identity = OptionContractIdentity {
        root: "SPX".into(),
        occ_symbol: "SPX   260918C00100000".into(),
    };
    let mut value = contract(&identity.occ_symbol, OptionType::Call, 100.0, expiration);
    value.contract_specification = None;
    let storage_calls = Arc::new(AtomicUsize::new(0));
    let app = GammaExposureApplication::new(
        Calendar(true),
        Intraday {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Ok(provider_snapshot(vec![OptionChain {
                root: identity.root,
                contratos: vec![value],
            }])),
        },
        Stored {
            calls: storage_calls.clone(),
            result: Ok(None),
        },
        Specifications {
            calls: Arc::new(AtomicUsize::new(0)),
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Err(PortError::Unavailable("catalog unavailable".into())),
        },
        YieldCurves,
    );
    assert_eq!(
        app.gamma_exposure(request("SPX")).await,
        Err(PortError::Unavailable("catalog unavailable".into()))
    );
    assert_eq!(storage_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn non_finite_intermediates_and_aggregate_overflow_are_excluded() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let mut intermediate = contract("OVERFLOW", OptionType::Call, 90.0, expiration);
    intermediate.gamma = Some(f64::MAX);
    intermediate.open_interest = Some(f64::MAX);
    let mut first_sum = contract("SUM-1", OptionType::Call, 100.0, expiration);
    first_sum.gamma = Some(1.0e308);
    first_sum.contract_specification = Some(specification("SPX", 100.0, "USD"));
    first_sum.open_interest = Some(1.0);
    let mut second_sum = first_sum.clone();
    second_sum.occ_symbol = "SUM-2".into();
    let mut input = snapshot(vec![intermediate, first_sum, second_sum]);
    input.underlying_price.as_mut().unwrap().value = 1.0;
    let result = calculate(&input, SnapshotOrigin::Intraday);

    assert_eq!(result.diagnostics.included_contracts, 1);
    assert_eq!(
        result.diagnostics.excluded_by_reason[&ExclusionReason::NumericOverflow],
        2
    );
    assert!(result.calls_gex.is_finite());
    assert!(result.puts_gex.is_finite());
    assert!(result.net_gex.is_finite());
    assert!(
        result
            .by_strike
            .iter()
            .all(|bucket| bucket.calls_gex.is_finite()
                && bucket.puts_gex.is_finite()
                && bucket.net_gex.is_finite())
    );
    assert!(
        result
            .by_expiration
            .iter()
            .all(|bucket| bucket.calls_gex.is_finite()
                && bucket.puts_gex.is_finite()
                && bucket.net_gex.is_finite())
    );
}

#[tokio::test]
async fn empty_or_fully_excluded_snapshots_are_unavailable_instead_of_factual_zero() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let empty = snapshot(Vec::new());
    let mut no_multiplier = contract("NO-MULTIPLIER", OptionType::Call, 100.0, expiration);
    no_multiplier.contract_specification = None;
    let mut no_gamma = contract("NO-GAMMA", OptionType::Call, 100.0, expiration);
    no_gamma.gamma = None;
    let mut no_interest = contract("NO-OI", OptionType::Call, 100.0, expiration);
    no_interest.open_interest = None;
    let mut overflow = contract("OVERFLOW", OptionType::Call, 100.0, expiration);
    overflow.gamma = Some(f64::MAX);
    overflow.open_interest = Some(f64::MAX);
    let mut no_spot = snapshot(vec![contract(
        "NO-SPOT",
        OptionType::Call,
        100.0,
        expiration,
    )]);
    no_spot.underlying_price = None;

    for unavailable in [
        empty,
        snapshot(vec![no_gamma]),
        snapshot(vec![no_interest]),
        snapshot(vec![no_multiplier]),
        snapshot(vec![overflow]),
        no_spot,
    ] {
        let (app, calls) = applications(
            false,
            Err(PortError::Unavailable("must not call".into())),
            Ok(Some(stored_snapshot(unavailable))),
        );
        assert!(matches!(
            app.gamma_exposure(request("SPX")).await,
            Err(PortError::Unavailable(message)) if message.contains("no contracts are eligible")
        ));
        assert_eq!(calls.provider.load(Ordering::SeqCst), 0);
        assert_eq!(calls.resolver.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn factual_zero_and_call_put_cancellation_remain_calculable() {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let mut zero_gamma = contract("ZERO-GAMMA", OptionType::Call, 90.0, expiration);
    zero_gamma.gamma = Some(0.0);
    let mut zero_interest = contract("ZERO-OI", OptionType::Call, 95.0, expiration);
    zero_interest.open_interest = Some(0.0);
    let cancelling = vec![
        contract("CALL", OptionType::Call, 100.0, expiration),
        contract("PUT", OptionType::Put, 100.0, expiration),
    ];

    for (contracts, included) in [
        (vec![zero_gamma], 1),
        (vec![zero_interest], 1),
        (cancelling, 2),
    ] {
        let (app, _) = applications(
            false,
            Err(PortError::Unavailable("must not call".into())),
            Ok(Some(stored_snapshot(snapshot(contracts)))),
        );
        let result = app.gamma_exposure(request("SPX")).await.unwrap();
        assert_eq!(
            result.current_exposure.diagnostics.included_contracts,
            included
        );
        assert_eq!(result.current_exposure.net_gex, 0.0);
    }
}

#[tokio::test]
async fn offsetless_provider_timestamp_does_not_expose_collection_time_as_as_of() {
    let identity = OptionContractIdentity {
        root: "SPX".into(),
        occ_symbol: "SPX   260918C00100000".into(),
    };
    let response: CboeResponse = serde_json::from_value(serde_json::json!({
        "timestamp": "2026-08-21 11:00:00",
        "data": {
            "current_price": 100.0,
            "options": [{
                "option": identity.occ_symbol.clone(), "bid": 1.0, "ask": 1.1,
                "volume": 1.0, "open_interest": 10.0, "delta": 0.5,
                "gamma": 0.02, "vega": 0.1, "theta": -0.1, "rho": 0.1,
                "theo": 1.0, "iv": 0.2
            }]
        }
    }))
    .unwrap();
    let collected_at = Utc.with_ymd_and_hms(2026, 8, 21, 15, 0, 1).unwrap();
    let provider_snapshot =
        response_to_snapshot_collected_at("SPX", response, collected_at).unwrap();
    assert_eq!(provider_snapshot.timestamp_utc, collected_at);
    let app = GammaExposureApplication::new(
        Calendar(true),
        Intraday {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Ok(provider_snapshot),
        },
        Stored {
            calls: Arc::new(AtomicUsize::new(0)),
            result: Ok(None),
        },
        Specifications {
            calls: Arc::new(AtomicUsize::new(0)),
            requested: Arc::new(Mutex::new(Vec::new())),
            result: Ok(BTreeMap::from([(
                identity,
                OptionContractSpecificationResolution::Found(specification("SPX", 100.0, "USD")),
            )])),
        },
        YieldCurves,
    );

    let result = app.gamma_exposure(request("SPX")).await.unwrap();
    assert_eq!(result.current_exposure.as_of, None);
    assert_ne!(result.current_exposure.as_of, Some(collected_at));
}

#[test]
fn public_dto_has_stable_json_names_and_explicit_methodology() {
    assert_eq!(
        serde_json::to_value(api_models::GammaExposureExclusionReason::InvalidImpliedVolatility)
            .unwrap(),
        serde_json::json!("invalid_implied_volatility")
    );
    let dto = api_models::GammaExposureResponse {
        current_exposure: api_models::CurrentGammaExposureResponse {
            ticker: "SPX".into(),
            spot: Some(100.0),
            currency: Some("USD".into()),
            as_of: Some(Utc.with_ymd_and_hms(2026, 8, 21, 20, 0, 0).unwrap()),
            snapshot_origin: api_models::GammaExposureSnapshotOrigin::EndOfDay,
            calls_gex: 2_000.0,
            puts_gex: -1_000.0,
            net_gex: 1_000.0,
            by_strike: Vec::new(),
            by_expiration: Vec::new(),
            methodology: "gamma × open_interest × contract_multiplier × spot² × 0.01".into(),
            sign_convention: "Analytical assumption: calls positive; puts negative.".into(),
            diagnostics: api_models::GammaExposureDiagnostics {
                total_contracts: 1,
                included_contracts: 1,
                excluded_contracts: 0,
                excluded_by_reason: Vec::new(),
                exclusion_samples: Vec::new(),
                exclusion_sample_limit: 20,
            },
        },
        modeled_profile: api_models::DataState::Available(
            api_models::ModeledGammaExposureProfile {
                valuation_time: Utc.with_ymd_and_hms(2026, 8, 21, 20, 0, 0).unwrap(),
                range_percent: 20.0,
                points: 81,
                methodology: "Black-Scholes gamma".into(),
                sticky_strike_assumption: "sticky-strike".into(),
                included_contracts: 1,
                excluded_contracts: 0,
                diagnostics: api_models::GammaExposureDiagnostics {
                    total_contracts: 1,
                    included_contracts: 1,
                    excluded_contracts: 0,
                    excluded_by_reason: Vec::new(),
                    exclusion_samples: Vec::new(),
                    exclusion_sample_limit: 20,
                },
                profile: vec![api_models::ModeledGammaExposurePoint {
                    spot: 100.0,
                    call_gex: 2_000.0,
                    put_gex: -1_000.0,
                    net_gex: 1_000.0,
                }],
                zero_crossings: Vec::new(),
                nearest_zero_crossing: None,
            },
        ),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["current_exposure"]["snapshot_origin"], "end_of_day");
    assert_eq!(json["current_exposure"]["spot"], 100.0);
    assert_eq!(json["modeled_profile"]["state"], "available");
    assert_eq!(json["modeled_profile"]["data"]["points"], 81);
    assert!(serde_json::from_value::<api_models::GammaExposureResponse>(json).is_ok());
}
