use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::synchronization::{
        OptionAnalysisCollaborators, SynchronizationApplication, SynchronizationStores,
    },
    domain::{
        index_history::IndexHistory, market_history::MarketHistory, options::Snapshot,
        treasury::YieldCurve, volatility::TermStructure,
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

#[async_trait]
impl ForLoadingTrackedTickers for TwoHistoryTickersMock {
    async fn load_active_tickers(
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
    async fn load_active_tickers(
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
    async fn load_active_tickers(
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
        _snapshot: &Snapshot,
        _market_close: chrono::DateTime<Utc>,
    ) -> PortResult<u64> {
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
        HistoryMock,
        OptionsMock,
        IndicesMock,
        CurvesMock,
        SynchronizationStores::new(
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            store,
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
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
        HistoryMock,
        OptionsMock,
        IndicesMock,
        CurvesMock,
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
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
        HistoryMock,
        OptionsMock,
        IndicesMock,
        CurvesMock,
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        TrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
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
        HistoryMock,
        OptionsSuccessMock,
        IndicesMock,
        CurvesMock,
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        OptionTrackedTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
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
        PartiallyFailingHistoryMock,
        OptionsMock,
        IndicesMock,
        CurvesMock,
        SynchronizationStores::new(
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
            StoreMock::default(),
        ),
        TwoHistoryTickersMock,
        OptionAnalysisCollaborators::new(OptionDataMock, OptionDataMock, TradingCalendarMock),
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
