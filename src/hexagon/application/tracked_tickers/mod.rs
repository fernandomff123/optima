//! Tracked-ticker configuration use cases.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::Mutex;

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::{
        ResolvedUnderlying, TrackedTicker, TrackedTickerConfiguration, UnderlyingResolutionState,
        is_system_ticker, normalize_ticker, system_tickers,
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
        for_resolving_underlyings::{ForResolvingUnderlyings, UnderlyingResolution},
    },
};

pub struct TrackedTickersApplication<Loader, Store, Resolver> {
    loader: Loader,
    store: Store,
    resolver: Resolver,
    configuration_coordinators: Mutex<HashMap<String, Arc<Mutex<u64>>>>,
}

impl<Loader, Store, Resolver> TrackedTickersApplication<Loader, Store, Resolver> {
    pub fn new(loader: Loader, store: Store, resolver: Resolver) -> Self {
        Self {
            loader,
            store,
            resolver,
            configuration_coordinators: Mutex::new(HashMap::new()),
        }
    }

    async fn coordinator_for(&self, ticker: &str) -> Arc<Mutex<u64>> {
        let mut coordinators = self.configuration_coordinators.lock().await;
        coordinators.retain(|_, coordinator| Arc::strong_count(coordinator) > 1);
        coordinators
            .entry(ticker.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(0)))
            .clone()
    }

    async fn release_coordinator(&self, ticker: &str, coordinator: &Arc<Mutex<u64>>) {
        let mut coordinators = self.configuration_coordinators.lock().await;
        if coordinators.get(ticker).is_some_and(|current| {
            Arc::ptr_eq(current, coordinator) && Arc::strong_count(current) == 2
        }) {
            coordinators.remove(ticker);
        }
    }
}

#[async_trait]
impl<Loader, Store, Resolver> ForManagingTrackedTickers
    for TrackedTickersApplication<Loader, Store, Resolver>
where
    Loader: ForLoadingTrackedTickers,
    Store: ForStoringTrackedTickers,
    Resolver: ForResolvingUnderlyingSymbols,
{
    async fn list_tickers(&self, include_inactive: bool) -> PortResult<Vec<TrackedTicker>> {
        if include_inactive {
            self.loader.load_tracked_tickers().await
        } else {
            self.loader.load_active_tickers().await
        }
    }

    async fn bootstrap_system_tickers(&self) -> PortResult<()> {
        let existing = self.loader.load_tracked_tickers().await?;
        for mut ticker in system_tickers() {
            if let Some(stored) = existing.iter().find(|stored| {
                stored.ticker == ticker.ticker
                    && stored.resolution_state == UnderlyingResolutionState::Resolved
            }) {
                ticker.validated_at = stored.validated_at;
                ticker.metadata = stored.metadata.clone();
            }
            self.store.store_tracked_ticker(&ticker).await?;
        }
        Ok(())
    }

    async fn configure_ticker(
        &self,
        ticker: &str,
        configuration: TrackedTickerConfiguration,
    ) -> PortResult<()> {
        let ticker = normalize_ticker(ticker).map_err(PortError::InvalidRequest)?;
        if is_system_ticker(&ticker) {
            if system_tickers()
                .into_iter()
                .find(|tracked| tracked.ticker == ticker)
                .is_some_and(|tracked| tracked.configuration() == configuration)
            {
                return Ok(());
            }
            return Err(PortError::Conflict(format!(
                "tracked ticker {ticker} is protected by the system"
            )));
        }
        let coordinator = self.coordinator_for(&ticker).await;
        let result = async {
            let (revision, existing) = {
                let mut current_revision = coordinator.lock().await;
                *current_revision += 1;
                let revision = *current_revision;
                let existing = self
                    .loader
                    .load_tracked_tickers()
                    .await?
                    .into_iter()
                    .find(|tracked| tracked.ticker == ticker);

                if let Some(mut tracked) = existing.clone() {
                    if !configuration.active
                        || tracked.resolution_state == UnderlyingResolutionState::Resolved
                    {
                        tracked.active = configuration.active;
                        tracked.historical_prices = configuration.historical_prices;
                        tracked.option_snapshots = configuration.option_snapshots;
                        self.store.store_tracked_ticker(&tracked).await?;
                        return Ok(());
                    }
                } else if !configuration.active {
                    let tracked = TrackedTicker::user(&ticker, configuration)
                        .map_err(PortError::InvalidRequest)?;
                    self.store.store_tracked_ticker(&tracked).await?;
                    return Ok(());
                }
                (revision, existing)
            };

            let resolution = self.resolver.resolve_underlying(&ticker).await;
            let current_revision = coordinator.lock().await;
            if *current_revision != revision {
                return Ok(());
            }

            let existing = self
                .loader
                .load_tracked_tickers()
                .await?
                .into_iter()
                .find(|tracked| tracked.ticker == ticker)
                .or(existing);

            if let Some(mut tracked) = existing {
                match resolution {
                    Ok(resolved) => {
                        let resolved = confirm_identity(&ticker, resolved)?;
                        tracked
                            .resolve(resolved, Utc::now())
                            .map_err(PortError::Unavailable)?;
                    }
                    Err(UnderlyingResolutionError::NotFound(message)) => {
                        tracked.reject();
                        tracked.active = configuration.active;
                        tracked.historical_prices = configuration.historical_prices;
                        tracked.option_snapshots = configuration.option_snapshots;
                        self.store.store_tracked_ticker(&tracked).await?;
                        return Err(PortError::NotFound(message));
                    }
                    Err(error) => return Err(map_resolution_error(error)),
                }
                tracked.active = configuration.active;
                tracked.historical_prices = configuration.historical_prices;
                tracked.option_snapshots = configuration.option_snapshots;
                return self.store.store_tracked_ticker(&tracked).await;
            }

            let resolved = resolution.map_err(map_resolution_error)?;
            let resolved = confirm_identity(&ticker, resolved)?;
            let mut tracked =
                TrackedTicker::user(&ticker, configuration).map_err(PortError::InvalidRequest)?;
            tracked
                .resolve(resolved, Utc::now())
                .map_err(PortError::Unavailable)?;
            self.store.store_tracked_ticker(&tracked).await
        }
        .await;
        self.release_coordinator(&ticker, &coordinator).await;
        result
    }
}

#[async_trait]
impl<Loader, Store, Resolver> ForResolvingUnderlyings
    for TrackedTickersApplication<Loader, Store, Resolver>
where
    Loader: ForLoadingTrackedTickers,
    Store: ForStoringTrackedTickers,
    Resolver: ForResolvingUnderlyingSymbols,
{
    async fn resolve_underlying(&self, ticker: &str) -> PortResult<UnderlyingResolution> {
        let ticker = normalize_ticker(ticker).map_err(PortError::InvalidRequest)?;
        let resolved = self
            .resolver
            .resolve_underlying(&ticker)
            .await
            .map_err(map_resolution_error)?;
        let ResolvedUnderlying { ticker, metadata } = confirm_identity(&ticker, resolved)?;
        Ok(UnderlyingResolution {
            ticker,
            validated_at: Utc::now(),
            metadata,
        })
    }
}

fn confirm_identity(
    requested_ticker: &str,
    resolved: ResolvedUnderlying,
) -> PortResult<ResolvedUnderlying> {
    let resolved_ticker = normalize_ticker(&resolved.ticker).map_err(|_| {
        PortError::Unavailable("provider returned an invalid underlying identity".into())
    })?;
    if resolved_ticker != requested_ticker {
        return Err(PortError::Unavailable(format!(
            "provider returned {resolved_ticker} while resolving {requested_ticker}"
        )));
    }
    Ok(ResolvedUnderlying {
        ticker: resolved_ticker,
        metadata: resolved.metadata,
    })
}

fn map_resolution_error(error: UnderlyingResolutionError) -> PortError {
    match error {
        UnderlyingResolutionError::NotFound(message) => PortError::NotFound(message),
        UnderlyingResolutionError::TemporarilyUnavailable(message)
        | UnderlyingResolutionError::InvalidProviderResponse(message) => {
            PortError::Unavailable(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::domain::tracked_ticker::UnderlyingMetadata;

    #[derive(Clone, Default)]
    struct EmptyTrackedTickers;

    #[async_trait]
    impl ForLoadingTrackedTickers for EmptyTrackedTickers {
        async fn load_tracked_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
            Ok(Vec::new())
        }

        async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
            Ok(Vec::new())
        }

        async fn load_refresh_eligible_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
            Ok(Vec::new())
        }
    }

    #[async_trait]
    impl ForStoringTrackedTickers for EmptyTrackedTickers {
        async fn store_tracked_ticker(&self, _ticker: &TrackedTicker) -> PortResult<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct BlockingResolver {
        entered: Arc<tokio::sync::Notify>,
    }

    #[async_trait]
    impl ForResolvingUnderlyingSymbols for BlockingResolver {
        async fn resolve_underlying(
            &self,
            ticker: &str,
        ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
            self.entered.notify_one();
            std::future::pending::<()>().await;
            Ok(ResolvedUnderlying {
                ticker: ticker.to_string(),
                metadata: UnderlyingMetadata::default(),
            })
        }
    }

    #[tokio::test]
    async fn obsolete_per_ticker_coordinators_are_removed_after_the_last_request() {
        let application = TrackedTickersApplication::new((), (), ());
        let first = application.coordinator_for("MSFT").await;
        let second = application.coordinator_for("MSFT").await;

        application.release_coordinator("MSFT", &first).await;
        assert_eq!(application.configuration_coordinators.lock().await.len(), 1);

        drop(first);
        application.release_coordinator("MSFT", &second).await;
        assert!(
            application
                .configuration_coordinators
                .lock()
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn later_configuration_cleans_a_cancelled_request_without_removing_active_ones() {
        let resolver = BlockingResolver::default();
        let application = Arc::new(TrackedTickersApplication::new(
            EmptyTrackedTickers,
            EmptyTrackedTickers,
            resolver.clone(),
        ));
        let configuring = {
            let application = application.clone();
            tokio::spawn(async move {
                application
                    .configure_ticker(
                        "MSFT",
                        TrackedTickerConfiguration {
                            active: true,
                            historical_prices: true,
                            option_snapshots: false,
                        },
                    )
                    .await
            })
        };
        resolver.entered.notified().await;
        configuring.abort();
        assert!(configuring.await.unwrap_err().is_cancelled());
        assert!(
            application
                .configuration_coordinators
                .lock()
                .await
                .contains_key("MSFT")
        );

        application
            .configure_ticker(
                "AAPL",
                TrackedTickerConfiguration {
                    active: false,
                    historical_prices: true,
                    option_snapshots: false,
                },
            )
            .await
            .unwrap();
        assert!(
            application
                .configuration_coordinators
                .lock()
                .await
                .is_empty()
        );

        let active = application.coordinator_for("MSFT").await;
        application
            .configure_ticker(
                "AAPL",
                TrackedTickerConfiguration {
                    active: false,
                    historical_prices: true,
                    option_snapshots: false,
                },
            )
            .await
            .unwrap();
        assert!(
            application
                .configuration_coordinators
                .lock()
                .await
                .contains_key("MSFT")
        );
        application.release_coordinator("MSFT", &active).await;
        assert!(
            application
                .configuration_coordinators
                .lock()
                .await
                .is_empty()
        );
    }
}
