//! Tracked-ticker configuration use cases.

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::tracked_ticker::TrackedTicker,
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
    async fn list_active_tickers(&self) -> PortResult<Vec<TrackedTicker>> {
        self.loader.load_active_tickers().await
    }

    async fn configure_ticker(&self, mut ticker: TrackedTicker) -> PortResult<()> {
        ticker.ticker = ticker.ticker.trim().to_ascii_uppercase();
        if ticker.ticker.is_empty()
            || !ticker
                .ticker
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '^')
        {
            return Err(PortError::InvalidRequest("invalid tracked ticker".into()));
        }
        self.store.store_tracked_ticker(&ticker).await
    }
}
