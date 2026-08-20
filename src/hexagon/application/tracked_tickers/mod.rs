//! Tracked-ticker configuration use cases.

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
    configuration_lock: Mutex<()>,
}

impl<Loader, Store, Resolver> TrackedTickersApplication<Loader, Store, Resolver> {
    pub fn new(loader: Loader, store: Store, resolver: Resolver) -> Self {
        Self {
            loader,
            store,
            resolver,
            configuration_lock: Mutex::new(()),
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
        let _configuration = self.configuration_lock.lock().await;
        let existing = self
            .loader
            .load_tracked_tickers()
            .await?
            .into_iter()
            .find(|tracked| tracked.ticker == ticker);

        if let Some(mut tracked) = existing {
            if !configuration.active
                || tracked.resolution_state == UnderlyingResolutionState::Resolved
            {
                tracked.active = configuration.active;
                tracked.historical_prices = configuration.historical_prices;
                tracked.option_snapshots = configuration.option_snapshots;
                return self.store.store_tracked_ticker(&tracked).await;
            }

            match self.resolver.resolve_underlying(&ticker).await {
                Ok(resolved) => tracked.resolve(resolved, Utc::now()),
                Err(UnderlyingResolutionError::NotFound(message)) => {
                    tracked.reject();
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

        let resolved = self
            .resolver
            .resolve_underlying(&ticker)
            .await
            .map_err(map_resolution_error)?;
        let mut tracked =
            TrackedTicker::user(&ticker, configuration).map_err(PortError::InvalidRequest)?;
        tracked.resolve(resolved, Utc::now());
        self.store.store_tracked_ticker(&tracked).await
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
        let ResolvedUnderlying { ticker, metadata } = self
            .resolver
            .resolve_underlying(&ticker)
            .await
            .map_err(map_resolution_error)?;
        Ok(UnderlyingResolution {
            ticker,
            validated_at: Utc::now(),
            metadata,
        })
    }
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
