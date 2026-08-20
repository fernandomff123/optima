use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use hexagonal_backend::hexagon::{
    PortError, PortResult,
    application::tracked_tickers::TrackedTickersApplication,
    domain::tracked_ticker::{
        ResolvedUnderlying, TrackedTicker, UnderlyingMetadata, UnderlyingResolutionState,
    },
    driven_ports::{
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_resolving_underlying_symbols::{
            ForResolvingUnderlyingSymbols, UnderlyingResolutionError,
        },
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
    driving_ports::{
        for_managing_tracked_tickers::ForManagingTrackedTickers,
        for_resolving_underlyings::ForResolvingUnderlyings,
    },
};

#[derive(Clone, Default)]
struct TrackedTickersMock(Arc<Mutex<Vec<TrackedTicker>>>);

#[derive(Clone, Copy)]
struct ResolverMock;

struct MissingResolver;
struct UnavailableResolver;
struct InvalidResponseResolver;
struct DifferentIdentityResolver;
struct MustNotResolve;

#[derive(Clone, Default)]
struct ControlledResolver {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    calls: Arc<AtomicUsize>,
}

impl ControlledResolver {
    async fn wait_until_calls(&self, expected: usize) {
        while self.calls.load(Ordering::SeqCst) < expected {
            self.entered.notified().await;
        }
    }

    fn release(&self) {
        self.release.notify_one();
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for ResolverMock {
    async fn resolve_underlying(
        &self,
        ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        Ok(ResolvedUnderlying {
            ticker: ticker.to_string(),
            metadata: UnderlyingMetadata::default(),
        })
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for MissingResolver {
    async fn resolve_underlying(
        &self,
        ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        Err(UnderlyingResolutionError::NotFound(format!(
            "underlying {ticker} was not found"
        )))
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for UnavailableResolver {
    async fn resolve_underlying(
        &self,
        _ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        Err(UnderlyingResolutionError::TemporarilyUnavailable(
            "Yahoo timeout".into(),
        ))
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for InvalidResponseResolver {
    async fn resolve_underlying(
        &self,
        _ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        Err(UnderlyingResolutionError::InvalidProviderResponse(
            "Yahoo response was incompatible".into(),
        ))
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for DifferentIdentityResolver {
    async fn resolve_underlying(
        &self,
        _ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        Ok(ResolvedUnderlying {
            ticker: "AAPL".into(),
            metadata: UnderlyingMetadata::default(),
        })
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for MustNotResolve {
    async fn resolve_underlying(
        &self,
        _ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        panic!("resolved ticker updates and deactivation must not call the provider")
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for ControlledResolver {
    async fn resolve_underlying(
        &self,
        ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        self.release.notified().await;
        Ok(ResolvedUnderlying {
            ticker: ticker.to_string(),
            metadata: UnderlyingMetadata {
                currency: Some("USD".into()),
                exchange: Some("NMS".into()),
                timezone: Some("America/New_York".into()),
                instrument_type: Some("EQUITY".into()),
            },
        })
    }
}

#[async_trait]
impl ForLoadingTrackedTickers for TrackedTickersMock {
    async fn load_tracked_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        Ok(self.0.lock().expect("test mutex must be usable").clone())
    }

    async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        Ok(self
            .0
            .lock()
            .expect("test mutex must be usable")
            .iter()
            .filter(|ticker| ticker.active)
            .cloned()
            .collect())
    }

    async fn load_refresh_eligible_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        Ok(self
            .0
            .lock()
            .expect("test mutex must be usable")
            .iter()
            .filter(|ticker| {
                ticker.source
                    == hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::System
                    || (ticker.active
                        && ticker.resolution_state == UnderlyingResolutionState::Resolved)
            })
            .cloned()
            .collect())
    }
}

#[async_trait]
impl ForStoringTrackedTickers for TrackedTickersMock {
    async fn store_tracked_ticker(&self, ticker: &TrackedTicker) -> PortResult<()> {
        let mut stored = self.0.lock().expect("test mutex must be usable");
        stored.retain(|current| current.ticker != ticker.ticker);
        stored.push(ticker.clone());
        Ok(())
    }
}

#[tokio::test]
async fn manages_tracked_tickers_through_mocked_ports() {
    let adapter = TrackedTickersMock::default();
    let application = TrackedTickersApplication::new(adapter.clone(), adapter, ResolverMock);
    application
        .configure_ticker(
            " qqq ",
            hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
                active: true,
                historical_prices: true,
                option_snapshots: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        application.list_tickers(false).await.unwrap()[0].ticker,
        "QQQ"
    );
}

#[tokio::test]
async fn user_configuration_is_idempotent_and_inactive_tickers_remain_listed() {
    let adapter = TrackedTickersMock::default();
    let application =
        TrackedTickersApplication::new(adapter.clone(), adapter.clone(), ResolverMock);
    let configuration =
        hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
            active: false,
            historical_prices: true,
            option_snapshots: false,
        };

    application
        .configure_ticker(" qqq ", configuration)
        .await
        .unwrap();
    application
        .configure_ticker("QQQ", configuration)
        .await
        .unwrap();

    let listed = application.list_tickers(true).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].ticker, "QQQ");
    assert!(!listed[0].active);
}

#[tokio::test]
async fn rejects_invalid_and_system_tickers_factually() {
    let adapter = TrackedTickersMock::default();
    let system = hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers()[0].clone();
    adapter.store_tracked_ticker(&system).await.unwrap();
    let application = TrackedTickersApplication::new(adapter.clone(), adapter, ResolverMock);
    let configuration =
        hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
            active: false,
            historical_prices: false,
            option_snapshots: false,
        };

    assert_eq!(
        application
            .configure_ticker("bad ticker", configuration)
            .await,
        Err(PortError::InvalidRequest("invalid tracked ticker".into()))
    );
    application
        .configure_ticker("spx", system.configuration())
        .await
        .expect("identical system configuration must be idempotent");
    assert_eq!(
        application.configure_ticker("spx", configuration).await,
        Err(PortError::Conflict(
            "tracked ticker SPX is protected by the system".into()
        ))
    );
}

fn configuration(
    active: bool,
) -> hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
    hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
        active,
        historical_prices: true,
        option_snapshots: false,
    }
}

#[tokio::test]
async fn resolved_creation_is_eligible_and_records_validation() {
    let adapter = TrackedTickersMock::default();
    let application =
        TrackedTickersApplication::new(adapter.clone(), adapter.clone(), ResolverMock);

    application
        .configure_ticker("MSFT", configuration(true))
        .await
        .unwrap();

    let stored = adapter.load_tracked_tickers().await.unwrap().remove(0);
    assert_eq!(stored.resolution_state, UnderlyingResolutionState::Resolved);
    assert!(stored.validated_at.is_some());
    assert_eq!(
        adapter.load_refresh_eligible_tickers().await.unwrap(),
        vec![stored]
    );
}

#[tokio::test]
async fn nonexistent_and_transient_new_tickers_are_not_persisted_or_eligible() {
    let missing_store = TrackedTickersMock::default();
    let missing = TrackedTickersApplication::new(
        missing_store.clone(),
        missing_store.clone(),
        MissingResolver,
    );
    assert!(matches!(
        missing
            .configure_ticker("MISSING", configuration(true))
            .await,
        Err(PortError::NotFound(_))
    ));
    assert!(
        missing_store
            .load_tracked_tickers()
            .await
            .unwrap()
            .is_empty()
    );

    let unavailable_store = TrackedTickersMock::default();
    let unavailable = TrackedTickersApplication::new(
        unavailable_store.clone(),
        unavailable_store.clone(),
        UnavailableResolver,
    );
    assert!(matches!(
        unavailable
            .configure_ticker("MSFT", configuration(true))
            .await,
        Err(PortError::Unavailable(_))
    ));
    assert!(
        unavailable_store
            .load_refresh_eligible_tickers()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn only_conclusive_absence_rejects_an_existing_pending_ticker() {
    let missing_store = TrackedTickersMock::default();
    missing_store
        .store_tracked_ticker(&TrackedTicker::user("MISSING", configuration(true)).unwrap())
        .await
        .unwrap();
    let missing = TrackedTickersApplication::new(
        missing_store.clone(),
        missing_store.clone(),
        MissingResolver,
    );
    assert!(matches!(
        missing
            .configure_ticker("MISSING", configuration(true))
            .await,
        Err(PortError::NotFound(_))
    ));
    let rejected = missing_store
        .load_tracked_tickers()
        .await
        .unwrap()
        .remove(0);
    assert_eq!(
        rejected.resolution_state,
        UnderlyingResolutionState::Rejected
    );
    assert!(rejected.validated_at.is_none());

    for invalid_response in [false, true] {
        let store = TrackedTickersMock::default();
        store
            .store_tracked_ticker(&TrackedTicker::user("MSFT", configuration(true)).unwrap())
            .await
            .unwrap();
        let result = if invalid_response {
            TrackedTickersApplication::new(store.clone(), store.clone(), InvalidResponseResolver)
                .configure_ticker("MSFT", configuration(true))
                .await
        } else {
            TrackedTickersApplication::new(store.clone(), store.clone(), UnavailableResolver)
                .configure_ticker("MSFT", configuration(true))
                .await
        };
        assert!(matches!(result, Err(PortError::Unavailable(_))));
        let pending = store.load_tracked_tickers().await.unwrap().remove(0);
        assert_eq!(pending.resolution_state, UnderlyingResolutionState::Pending);
        assert!(pending.validated_at.is_none());
    }
}

#[tokio::test]
async fn resolved_ticker_can_be_updated_and_disabled_without_provider() {
    let adapter = TrackedTickersMock::default();
    let mut tracked = TrackedTicker::user("MSFT", configuration(true)).unwrap();
    let validated_at = chrono::Utc::now();
    tracked
        .resolve(
            ResolvedUnderlying {
                ticker: "MSFT".into(),
                metadata: UnderlyingMetadata {
                    currency: Some("USD".into()),
                    exchange: Some("NMS".into()),
                    timezone: None,
                    instrument_type: Some("EQUITY".into()),
                },
            },
            validated_at,
        )
        .expect("matching resolution must succeed");
    adapter.store_tracked_ticker(&tracked).await.unwrap();
    let application =
        TrackedTickersApplication::new(adapter.clone(), adapter.clone(), MustNotResolve);

    application
        .configure_ticker("MSFT", configuration(true))
        .await
        .unwrap();
    application
        .configure_ticker("MSFT", configuration(true))
        .await
        .unwrap();
    application
        .configure_ticker("MSFT", configuration(false))
        .await
        .unwrap();

    let stored = adapter.load_tracked_tickers().await.unwrap().remove(0);
    assert!(!stored.active);
    assert_eq!(stored.resolution_state, UnderlyingResolutionState::Resolved);
    assert_eq!(stored.validated_at, Some(validated_at));
    assert_eq!(stored.metadata.currency.as_deref(), Some("USD"));
}

#[tokio::test]
async fn pending_existing_ticker_is_resolved_again_by_put() {
    let adapter = TrackedTickersMock::default();
    adapter
        .store_tracked_ticker(&TrackedTicker::user("MSFT", configuration(true)).unwrap())
        .await
        .unwrap();
    let application =
        TrackedTickersApplication::new(adapter.clone(), adapter.clone(), ResolverMock);

    application
        .configure_ticker("MSFT", configuration(true))
        .await
        .unwrap();

    assert_eq!(
        adapter.load_tracked_tickers().await.unwrap()[0].resolution_state,
        UnderlyingResolutionState::Resolved
    );
}

#[tokio::test]
async fn active_listing_does_not_confuse_user_intent_with_resolution() {
    let adapter = TrackedTickersMock::default();
    adapter
        .store_tracked_ticker(&TrackedTicker::user("PENDING", configuration(true)).unwrap())
        .await
        .unwrap();
    let application =
        TrackedTickersApplication::new(adapter.clone(), adapter.clone(), ResolverMock);

    assert_eq!(application.list_tickers(false).await.unwrap().len(), 1);
    assert!(
        adapter
            .load_refresh_eligible_tickers()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn exact_resolution_use_case_does_not_persist_or_activate() {
    let adapter = TrackedTickersMock::default();
    let application =
        TrackedTickersApplication::new(adapter.clone(), adapter.clone(), ResolverMock);

    let resolved = application.resolve_underlying(" msft ").await.unwrap();

    assert_eq!(resolved.ticker, "MSFT");
    assert!(adapter.load_tracked_tickers().await.unwrap().is_empty());
}

#[tokio::test]
async fn different_resolved_identity_fails_without_altering_or_creating_records() {
    let empty_store = TrackedTickersMock::default();
    let application = TrackedTickersApplication::new(
        empty_store.clone(),
        empty_store.clone(),
        DifferentIdentityResolver,
    );
    assert!(matches!(
        application
            .configure_ticker("MSFT", configuration(true))
            .await,
        Err(PortError::Unavailable(_))
    ));
    assert!(empty_store.load_tracked_tickers().await.unwrap().is_empty());
    assert!(matches!(
        application.resolve_underlying("MSFT").await,
        Err(PortError::Unavailable(_))
    ));

    let existing_store = TrackedTickersMock::default();
    let original = TrackedTicker::user("MSFT", configuration(true)).unwrap();
    existing_store
        .store_tracked_ticker(&original)
        .await
        .unwrap();
    let application = TrackedTickersApplication::new(
        existing_store.clone(),
        existing_store.clone(),
        DifferentIdentityResolver,
    );
    assert!(matches!(
        application
            .configure_ticker("MSFT", configuration(true))
            .await,
        Err(PortError::Unavailable(_))
    ));
    assert_eq!(
        existing_store.load_tracked_tickers().await.unwrap(),
        vec![original]
    );
}

#[tokio::test]
async fn concurrent_puts_for_the_same_ticker_keep_the_latest_configuration() {
    let adapter = TrackedTickersMock::default();
    let resolver = ControlledResolver::default();
    let application = Arc::new(TrackedTickersApplication::new(
        adapter.clone(),
        adapter.clone(),
        resolver.clone(),
    ));
    let first = {
        let application = application.clone();
        tokio::spawn(async move {
            application
                .configure_ticker("MSFT", configuration(true))
                .await
        })
    };
    resolver.wait_until_calls(1).await;
    let second_configuration =
        hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerConfiguration {
            active: true,
            historical_prices: false,
            option_snapshots: true,
        };
    let second = {
        let application = application.clone();
        tokio::spawn(async move {
            application
                .configure_ticker("MSFT", second_configuration)
                .await
        })
    };
    resolver.wait_until_calls(2).await;
    resolver.release();
    resolver.release();
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();

    let stored = adapter.load_tracked_tickers().await.unwrap().remove(0);
    assert_eq!(stored.configuration(), second_configuration);
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn deactivation_during_resolution_wins_over_the_older_provider_response() {
    let adapter = TrackedTickersMock::default();
    adapter
        .store_tracked_ticker(&TrackedTicker::user("MSFT", configuration(true)).unwrap())
        .await
        .unwrap();
    let resolver = ControlledResolver::default();
    let application = Arc::new(TrackedTickersApplication::new(
        adapter.clone(),
        adapter.clone(),
        resolver.clone(),
    ));
    let resolving = {
        let application = application.clone();
        tokio::spawn(async move {
            application
                .configure_ticker("MSFT", configuration(true))
                .await
        })
    };
    resolver.wait_until_calls(1).await;
    let disabling = {
        let application = application.clone();
        tokio::spawn(async move {
            application
                .configure_ticker("MSFT", configuration(false))
                .await
        })
    };
    disabling.await.unwrap().unwrap();
    assert!(
        adapter
            .load_refresh_eligible_tickers()
            .await
            .unwrap()
            .is_empty(),
        "the stale provider response must not create an eligibility window"
    );
    resolver.release();
    resolving.await.unwrap().unwrap();

    let stored = adapter.load_tracked_tickers().await.unwrap().remove(0);
    assert!(!stored.active);
    assert_eq!(stored.resolution_state, UnderlyingResolutionState::Pending);
    assert_eq!(stored.metadata, UnderlyingMetadata::default());
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn different_tickers_do_not_block_each_other_during_resolution() {
    let adapter = TrackedTickersMock::default();
    let resolver = ControlledResolver::default();
    let application = Arc::new(TrackedTickersApplication::new(
        adapter.clone(),
        adapter.clone(),
        resolver.clone(),
    ));
    let msft = {
        let application = application.clone();
        tokio::spawn(async move {
            application
                .configure_ticker("MSFT", configuration(true))
                .await
        })
    };
    resolver.wait_until_calls(1).await;
    let aapl = {
        let application = application.clone();
        tokio::spawn(async move {
            application
                .configure_ticker("AAPL", configuration(true))
                .await
        })
    };
    resolver.wait_until_calls(2).await;

    resolver.release();
    resolver.release();
    msft.await.unwrap().unwrap();
    aapl.await.unwrap().unwrap();

    let mut tickers = adapter.load_tracked_tickers().await.unwrap();
    tickers.sort_by(|left, right| left.ticker.cmp(&right.ticker));
    assert_eq!(
        tickers
            .iter()
            .map(|ticker| ticker.ticker.as_str())
            .collect::<Vec<_>>(),
        vec!["AAPL", "MSFT"]
    );
    assert!(
        tickers
            .iter()
            .all(|ticker| ticker.resolution_state == UnderlyingResolutionState::Resolved)
    );
}
