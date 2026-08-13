//! Tracked-ticker configuration use cases.

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::{
        TrackedTicker, TrackedTickerConfiguration, is_system_ticker, normalize_ticker,
        system_tickers,
    },
    driven_ports::{
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
    driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers,
};

pub struct TrackedTickersApplication<Loader, Store> {
    loader: Loader,
    store: Store,
}

impl<Loader, Store> TrackedTickersApplication<Loader, Store> {
    pub fn new(loader: Loader, store: Store) -> Self {
        Self { loader, store }
    }
}

#[async_trait]
impl<Loader, Store> ForManagingTrackedTickers for TrackedTickersApplication<Loader, Store>
where
    Loader: ForLoadingTrackedTickers,
    Store: ForStoringTrackedTickers,
{
    async fn list_tickers(&self, include_inactive: bool) -> PortResult<Vec<TrackedTicker>> {
        if include_inactive {
            self.loader.load_tracked_tickers().await
        } else {
            self.loader.load_active_tickers().await
        }
    }

    async fn bootstrap_system_tickers(&self) -> PortResult<()> {
        for ticker in system_tickers() {
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
        let ticker =
            TrackedTicker::user(&ticker, configuration).map_err(PortError::InvalidRequest)?;
        self.store.store_tracked_ticker(&ticker).await
    }
}
