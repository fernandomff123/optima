use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::synchronization::{
        OptionAnalysisCollaborators, OptionSnapshotEnrichment, SynchronizationApplication,
        SynchronizationSources, SynchronizationStores,
    },
    domain::{
        index_history::IndexHistory,
        market_history::MarketHistory,
        options::{
            ContratoOpcao, OptionChain, OptionContractSpecification, OptionType, Snapshot,
            UnderlyingPriceObservation,
        },
        treasury::YieldCurve,
        volatility::TermStructure,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_obtaining_volatility_indices::ForObtainingVolatilityIndices,
        for_obtaining_yield_curves::ForObtainingYieldCurves,
        for_resolving_option_contract_specifications::{
            ForResolvingOptionContractSpecifications, OptionContractIdentity,
            OptionContractSpecificationResolution,
        },
        for_storing_index_history::ForStoringIndexHistory,
        for_storing_market_history::ForStoringMarketHistory,
        for_storing_option_chains::ForStoringOptionChains,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
    driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData,
};

struct HistoryMock;
struct OptionsMock;
struct IndicesMock;
struct CurvesMock;
struct TrackedTickersMock;
struct OptionDataMock;
struct TradingCalendarMock;
struct OptionTrackedTickersMock;
struct OptionsSuccessMock;
struct PartiallyFailingHistoryMock;
struct TwoHistoryTickersMock;
struct NoContractSpecifications;

#[derive(Clone)]
struct SnapshotOptionsMock(Snapshot);

#[async_trait]
impl ForObtainingOptionChains for SnapshotOptionsMock {
    async fn obtain_option_chain(&self, _ticker: &str) -> PortResult<Snapshot> {
        Ok(self.0.clone())
    }
}

#[derive(Clone)]
struct RecordingContractSpecifications {
    calls: Arc<AtomicUsize>,
    requested: Arc<Mutex<Vec<Vec<OptionContractIdentity>>>>,
    resolutions: BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>,
    fail: bool,
}

#[async_trait]
impl ForResolvingOptionContractSpecifications for RecordingContractSpecifications {
    async fn resolve_option_contract_specifications(
        &self,
        contracts: &[OptionContractIdentity],
    ) -> PortResult<BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.requested
            .lock()
            .expect("requested batches mutex must be usable")
            .push(contracts.to_vec());
        if self.fail {
            return Err(PortError::Unavailable("catalog unavailable".to_string()));
        }
        Ok(self.resolutions.clone())
    }
}

#[async_trait]
impl ForResolvingOptionContractSpecifications for NoContractSpecifications {
    async fn resolve_option_contract_specifications(
        &self,
        contracts: &[OptionContractIdentity],
    ) -> PortResult<BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>> {
        Ok(contracts
            .iter()
            .cloned()
            .map(|identity| (identity, OptionContractSpecificationResolution::NotFound))
            .collect())
    }
}

#[async_trait]
impl ForLoadingTrackedTickers for TwoHistoryTickersMock {
    async fn load_tracked_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        unreachable!("synchronization must only load active tracked tickers")
    }

    async fn load_active_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        unreachable!("synchronization must not use active-only loading")
    }

    async fn load_refresh_eligible_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        Ok(["XLE", "XLK"]
            .into_iter()
            .map(
                |ticker| hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker {
                    ticker: ticker.to_string(),
                    source: hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::User,
                    active: true,
                    historical_prices: true,
                    option_snapshots: false,
                    resolution_state: hexagonal_backend::hexagon::domain::tracked_ticker::UnderlyingResolutionState::Resolved,
                    validated_at: None,
                    metadata: hexagonal_backend::hexagon::domain::tracked_ticker::UnderlyingMetadata::default(),
                },
            )
            .collect())
    }
}

#[async_trait]
impl ForObtainingMarketHistory for PartiallyFailingHistoryMock {
    async fn obtain_market_history(
        &self,
        ticker: &str,
        _since: NaiveDate,
    ) -> PortResult<MarketHistory> {
        if ticker == "XLE" {
            return Err(PortError::Unavailable("provider failure".to_string()));
        }
        Ok(MarketHistory {
            ticker: ticker.to_string(),
            currency: Some("USD".to_string()),
            exchange_timezone: None,
            daily_quotes: Vec::new(),
            dividends: Vec::new(),
            splits: Vec::new(),
        })
    }
}

#[async_trait]
impl ForLoadingTrackedTickers for OptionTrackedTickersMock {
    async fn load_tracked_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        unreachable!("synchronization must only load active tracked tickers")
    }

    async fn load_active_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        unreachable!("synchronization must not use active-only loading")
    }

    async fn load_refresh_eligible_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        Ok(vec![
            hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker {
                ticker: "SPY".to_string(),
                source:
                    hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::User,
                active: true,
                historical_prices: false,
                option_snapshots: true,
                resolution_state: hexagonal_backend::hexagon::domain::tracked_ticker::UnderlyingResolutionState::Resolved,
                validated_at: None,
                metadata: hexagonal_backend::hexagon::domain::tracked_ticker::UnderlyingMetadata::default(),
            },
        ])
    }
}

#[async_trait]
impl ForObtainingOptionChains for OptionsSuccessMock {
    async fn obtain_option_chain(&self, ticker: &str) -> PortResult<Snapshot> {
        Ok(Snapshot {
            ticker: ticker.to_string(),
            timestamp_utc: Utc::now(),
            contratos: Vec::new(),
            chains: Vec::new(),
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: Default::default(),
        })
    }
}

#[async_trait]
impl ForLoadingOptionChains for OptionDataMock {
    async fn load_option_chain(&self, _ticker: &str) -> PortResult<Option<Snapshot>> {
        Ok(None)
    }
}

#[async_trait]
impl ForLoadingVolatilityTermStructures for OptionDataMock {
    async fn load_term_structure(&self, _ticker: &str) -> PortResult<Option<TermStructure>> {
        Ok(None)
    }

    async fn load_term_structure_at_or_before(
        &self,
        _ticker: &str,
        _instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>> {
        Ok(None)
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
impl ForLoadingYieldCurves for OptionDataMock {
    async fn load_yield_curve(&self, _on_or_before: NaiveDate) -> PortResult<Option<YieldCurve>> {
        Ok(None)
    }
}

impl ForConsultingTradingCalendar for TradingCalendarMock {
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

#[async_trait]
impl ForLoadingTrackedTickers for TrackedTickersMock {
    async fn load_tracked_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        unreachable!("synchronization must only load active tracked tickers")
    }

    async fn load_active_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        unreachable!("synchronization must not use active-only loading")
    }

    async fn load_refresh_eligible_tickers(
        &self,
    ) -> PortResult<Vec<hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker>> {
        Ok(vec![
            hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTicker {
                ticker: "SPY".to_string(),
                source:
                    hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::User,
                active: true,
                historical_prices: true,
                option_snapshots: false,
                resolution_state: hexagonal_backend::hexagon::domain::tracked_ticker::UnderlyingResolutionState::Resolved,
                validated_at: None,
                metadata: hexagonal_backend::hexagon::domain::tracked_ticker::UnderlyingMetadata::default(),
            },
        ])
    }
}

#[async_trait]
impl ForObtainingMarketHistory for HistoryMock {
    async fn obtain_market_history(
        &self,
        ticker: &str,
        _since: NaiveDate,
    ) -> PortResult<MarketHistory> {
        Ok(MarketHistory {
            ticker: ticker.to_string(),
            currency: Some("USD".to_string()),
            exchange_timezone: None,
            daily_quotes: Vec::new(),
            dividends: Vec::new(),
            splits: Vec::new(),
        })
    }
}

#[async_trait]
impl ForObtainingOptionChains for OptionsMock {
    async fn obtain_option_chain(&self, _ticker: &str) -> PortResult<Snapshot> {
        Err(PortError::Unavailable("unused option mock".to_string()))
    }
}

#[async_trait]
impl ForObtainingVolatilityIndices for IndicesMock {
    async fn obtain_volatility_index(&self, _ticker: &str) -> PortResult<IndexHistory> {
        Err(PortError::Unavailable("unused index mock".to_string()))
    }
}

#[async_trait]
impl ForObtainingYieldCurves for CurvesMock {
    async fn obtain_yield_curves(&self, _year: i32) -> PortResult<Vec<YieldCurve>> {
        Err(PortError::Unavailable("unused curve mock".to_string()))
    }
}

#[derive(Clone, Default)]
struct StoreMock {
    stored_tickers: Arc<Mutex<Vec<String>>>,
    stored_option_snapshots: Arc<Mutex<Vec<Snapshot>>>,
}

#[async_trait]
impl ForStoringMarketHistory for StoreMock {
    async fn store_market_history(&self, history: &MarketHistory) -> PortResult<u64> {
        self.stored_tickers
            .lock()
            .expect("test mutex must be usable")
            .push(history.ticker.clone());
        Ok(1)
    }
}

#[async_trait]
impl ForStoringOptionChains for StoreMock {
    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        _market_close: chrono::DateTime<Utc>,
    ) -> PortResult<u64> {
        self.stored_option_snapshots
            .lock()
            .expect("stored option snapshots mutex must be usable")
            .push(snapshot.clone());
        Ok(1)
    }
}

#[async_trait]
impl ForStoringVolatilityTermStructures for StoreMock {
    async fn store_term_structure(&self, _term_structure: &TermStructure) -> PortResult<u64> {
        Ok(1)
    }
}

#[async_trait]
impl ForStoringIndexHistory for StoreMock {
    async fn store_index_history(&self, _history: &IndexHistory) -> PortResult<u64> {
        Ok(1)
    }
}

#[async_trait]
impl ForStoringYieldCurves for StoreMock {
    async fn store_yield_curves(&self, _curves: &[YieldCurve]) -> PortResult<u64> {
        Ok(1)
    }
}

#[tokio::test]
async fn driving_port_orchestrates_provider_and_store_mocks() {
    let store = StoreMock::default();
    let observed = Arc::clone(&store.stored_tickers);
    let application = SynchronizationApplication::new(
        SynchronizationSources::new(HistoryMock, OptionsMock, IndicesMock, CurvesMock),
        SynchronizationStores::new(
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            store,
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(NoContractSpecifications),
    );

    let report = application
        .synchronize_market_history(
            " spy ",
            NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid test date"),
        )
        .await
        .expect("mock synchronization must succeed");

    assert_eq!(report.items_stored, 1);
    assert_eq!(
        *observed.lock().expect("test mutex must be usable"),
        vec!["SPY"]
    );
}

#[tokio::test]
async fn invalid_year_is_rejected_before_calling_driven_ports() {
    let application = SynchronizationApplication::new(
        SynchronizationSources::new(HistoryMock, OptionsMock, IndicesMock, CurvesMock),
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(NoContractSpecifications),
    );

    let error = application
        .synchronize_yield_curves(1800)
        .await
        .expect_err("invalid year must be rejected");

    assert_eq!(
        error,
        PortError::InvalidRequest("invalid year: 1800".into())
    );
}

#[tokio::test]
async fn batch_synchronization_uses_tracked_ticker_configuration() {
    use hexagonal_backend::hexagon::driving_ports::for_synchronizing_market_data::SynchronizeTrackedTickers;

    let application = SynchronizationApplication::new(
        SynchronizationSources::new(HistoryMock, OptionsMock, IndicesMock, CurvesMock),
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(NoContractSpecifications),
    );
    let report = application
        .synchronize_tracked_tickers(SynchronizeTrackedTickers {
            since: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            market_close: Utc::now(),
        })
        .await
        .unwrap();

    assert_eq!(report.tickers, 1);
    assert!(report.failures.is_empty());
    assert_eq!(report.items_stored, 1);
}

#[tokio::test]
async fn batch_reports_term_structure_separately_after_storing_option_chain() {
    use hexagonal_backend::hexagon::driving_ports::for_synchronizing_market_data::SynchronizeTrackedTickers;

    let application = SynchronizationApplication::new(
        SynchronizationSources::new(HistoryMock, OptionsSuccessMock, IndicesMock, CurvesMock),
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        OptionTrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(NoContractSpecifications),
    );
    let report = application
        .synchronize_tracked_tickers(SynchronizeTrackedTickers {
            since: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            market_close: Utc::now(),
        })
        .await
        .expect("batch must aggregate individual failures");

    assert_eq!(report.items_stored, 1);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].operation, "term_structure");
}

#[tokio::test]
async fn batch_isolates_one_ticker_history_failure_and_continues() {
    let application = SynchronizationApplication::new(
        SynchronizationSources::new(
            PartiallyFailingHistoryMock,
            OptionsMock,
            IndicesMock,
            CurvesMock,
        ),
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        TwoHistoryTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(NoContractSpecifications),
    );

    let report = application
        .synchronize_tracked_tickers(
            hexagonal_backend::hexagon::driving_ports::for_synchronizing_market_data::SynchronizeTrackedTickers {
                since: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
                market_close: Utc::now(),
            },
        )
        .await
        .expect("batch must aggregate ticker failures");

    assert_eq!(report.tickers, 2);
    assert_eq!(report.items_stored, 1);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].ticker, "XLE");
    assert_eq!(report.failures[0].operation, "market_history");
}

fn option_contract(occ_symbol: &str) -> ContratoOpcao {
    ContratoOpcao {
        occ_symbol: occ_symbol.to_string(),
        option_type: OptionType::Call,
        strike: 5_000.0,
        expiration: NaiveDate::from_ymd_opt(2026, 9, 18).expect("valid expiration"),
        bid: 1.0,
        ask: 1.2,
        mid: 1.1,
        spread: 0.2,
        volume: 10.0,
        open_interest: 20.0,
        delta: 0.5,
        gamma: 0.02,
        vega: 0.1,
        theta: -0.03,
        rho: 0.01,
        theo: 1.1,
        implied_volatility: Some(0.2),
        contract_specification: None,
    }
}

fn option_snapshot(chains: Vec<OptionChain>) -> Snapshot {
    Snapshot {
        ticker: "SPX".to_string(),
        timestamp_utc: DateTime::from_timestamp(1_776_000_000, 0).expect("valid timestamp"),
        contratos: Vec::new(),
        chains,
        underlying_price: UnderlyingPriceObservation::new(
            5_000.0,
            None,
            None,
            "provider-neutral-test",
        ),
        collected_at: Some(
            DateTime::from_timestamp(1_776_000_001, 0).expect("valid collection timestamp"),
        ),
        provider_timestamp: None,
        ingestion_diagnostics: Default::default(),
    }
}

fn specification(root: &str, multiplier: f64, currency: &str) -> OptionContractSpecification {
    OptionContractSpecification::new(
        root,
        multiplier,
        currency,
        "test-catalog",
        NaiveDate::from_ymd_opt(2026, 8, 21),
        None,
    )
    .expect("valid test specification")
}

#[tokio::test]
async fn option_snapshot_resolves_one_unique_batch_and_enriches_each_identity() {
    let standard = OptionContractIdentity {
        root: "SPX".to_string(),
        occ_symbol: "SPX   260918C05000000".to_string(),
    };
    let special = OptionContractIdentity {
        root: "SPX".to_string(),
        occ_symbol: "SPX   260918C05100000".to_string(),
    };
    let adjusted = OptionContractIdentity {
        root: "SPX1".to_string(),
        occ_symbol: "SPX1  260918C05000000".to_string(),
    };
    let resolutions = BTreeMap::from([
        (
            standard.clone(),
            OptionContractSpecificationResolution::Found(specification("SPX", 100.0, "USD")),
        ),
        (
            special.clone(),
            OptionContractSpecificationResolution::Found(specification("SPX", 50.0, "EUR")),
        ),
        (
            adjusted.clone(),
            OptionContractSpecificationResolution::NotFound,
        ),
    ]);
    let calls = Arc::new(AtomicUsize::new(0));
    let requested = Arc::new(Mutex::new(Vec::new()));
    let resolver = RecordingContractSpecifications {
        calls: Arc::clone(&calls),
        requested: Arc::clone(&requested),
        resolutions,
        fail: false,
    };
    let snapshot = option_snapshot(vec![
        OptionChain {
            root: "SPX".to_string(),
            contratos: vec![
                option_contract(&standard.occ_symbol),
                option_contract(&special.occ_symbol),
                option_contract(&standard.occ_symbol),
            ],
        },
        OptionChain {
            root: "SPX1".to_string(),
            contratos: vec![option_contract(&adjusted.occ_symbol)],
        },
    ]);
    let store = StoreMock::default();
    let stored = Arc::clone(&store.stored_option_snapshots);
    let application = SynchronizationApplication::new(
        SynchronizationSources::new(
            HistoryMock,
            SnapshotOptionsMock(snapshot),
            IndicesMock,
            CurvesMock,
        ),
        SynchronizationStores::new(
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            store,
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(resolver),
    );

    application
        .synchronize_option_chain(
            "SPX",
            DateTime::from_timestamp(1_776_000_000, 0).expect("valid market close"),
        )
        .await
        .expect("batch enrichment must succeed");

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let batches = requested
        .lock()
        .expect("requested batches mutex must be usable");
    assert_eq!(batches.len(), 1);
    assert_eq!(
        batches[0],
        vec![standard.clone(), special.clone(), adjusted]
    );
    drop(batches);

    let snapshots = stored
        .lock()
        .expect("stored option snapshots mutex must be usable");
    let stored = &snapshots[0];
    assert_eq!(stored.underlying_price.as_ref().unwrap().currency, None);
    assert_eq!(
        stored.chains[0].contratos[0]
            .contract_specification
            .as_ref()
            .unwrap()
            .contract_multiplier,
        100.0
    );
    assert_eq!(
        stored.chains[0].contratos[1]
            .contract_specification
            .as_ref()
            .unwrap()
            .contract_multiplier,
        50.0
    );
    assert_eq!(
        stored.chains[0].contratos[2].contract_specification,
        stored.chains[0].contratos[0].contract_specification
    );
    assert!(
        stored.chains[1].contratos[0]
            .contract_specification
            .is_none()
    );
}

#[tokio::test]
async fn option_snapshot_propagates_resolver_failure_without_storing() {
    let calls = Arc::new(AtomicUsize::new(0));
    let resolver = RecordingContractSpecifications {
        calls: Arc::clone(&calls),
        requested: Arc::new(Mutex::new(Vec::new())),
        resolutions: BTreeMap::new(),
        fail: true,
    };
    let snapshot = option_snapshot(vec![OptionChain {
        root: "SPX".to_string(),
        contratos: vec![option_contract("SPX   260918C05000000")],
    }]);
    let store = StoreMock::default();
    let stored = Arc::clone(&store.stored_option_snapshots);
    let application = SynchronizationApplication::new(
        SynchronizationSources::new(
            HistoryMock,
            SnapshotOptionsMock(snapshot),
            IndicesMock,
            CurvesMock,
        ),
        SynchronizationStores::new(
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            store,
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
        OptionSnapshotEnrichment::new(resolver),
    );

    let error = application
        .synchronize_option_chain(
            "SPX",
            DateTime::from_timestamp(1_776_000_000, 0).expect("valid market close"),
        )
        .await
        .expect_err("resolver failure must remain a failure");

    assert!(matches!(error, PortError::Unavailable(message) if message == "catalog unavailable"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        stored
            .lock()
            .expect("stored option snapshots mutex must be usable")
            .is_empty()
    );
}

#[tokio::test]
async fn option_snapshot_rejects_missing_and_additional_batch_identities_before_storing() {
    let requested_a = OptionContractIdentity {
        root: "SPX".to_string(),
        occ_symbol: "SPX   260918C05000000".to_string(),
    };
    let requested_b = OptionContractIdentity {
        root: "SPXW".to_string(),
        occ_symbol: "SPXW  260918C05100000".to_string(),
    };
    let additional = OptionContractIdentity {
        root: "XSP".to_string(),
        occ_symbol: "XSP   260918C00500000".to_string(),
    };
    let snapshot = option_snapshot(vec![
        OptionChain {
            root: requested_a.root.clone(),
            contratos: vec![option_contract(&requested_a.occ_symbol)],
        },
        OptionChain {
            root: requested_b.root.clone(),
            contratos: vec![option_contract(&requested_b.occ_symbol)],
        },
    ]);
    let incompatible_responses = [
        BTreeMap::from([(
            requested_a.clone(),
            OptionContractSpecificationResolution::Found(specification("SPX", 100.0, "USD")),
        )]),
        BTreeMap::from([
            (
                requested_a.clone(),
                OptionContractSpecificationResolution::Found(specification("SPX", 100.0, "USD")),
            ),
            (
                requested_b.clone(),
                OptionContractSpecificationResolution::NotFound,
            ),
            (additional, OptionContractSpecificationResolution::NotFound),
        ]),
    ];

    for resolutions in incompatible_responses {
        let store = StoreMock::default();
        let stored = Arc::clone(&store.stored_option_snapshots);
        let resolver = RecordingContractSpecifications {
            calls: Arc::new(AtomicUsize::new(0)),
            requested: Arc::new(Mutex::new(Vec::new())),
            resolutions,
            fail: false,
        };
        let application = SynchronizationApplication::new(
            SynchronizationSources::new(
                HistoryMock,
                SnapshotOptionsMock(snapshot.clone()),
                IndicesMock,
                CurvesMock,
            ),
            SynchronizationStores::new(
                store.clone(),
                store.clone(),
                store.clone(),
                store.clone(),
                store,
            ),
            TrackedTickersMock,
            OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
            OptionSnapshotEnrichment::new(resolver),
        );

        let error = application
            .synchronize_option_chain(
                "SPX",
                DateTime::from_timestamp(1_776_000_000, 0).expect("valid market close"),
            )
            .await
            .expect_err("incompatible identity set must fail");

        assert!(matches!(
            error,
            PortError::Unavailable(message)
                if message == "contract specification resolver returned an incompatible identity set"
        ));
        assert!(
            stored
                .lock()
                .expect("stored option snapshots mutex must be usable")
                .is_empty()
        );
    }
}
