use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use polars_options::hexagon::{
    PortResult,
    application::tracked_tickers::TrackedTickersApplication,
    domain::tracked_ticker::TrackedTicker,
    driven_ports::{
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
    driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers,
};

#[derive(Clone, Default)]
struct TrackedTickersMock(Arc<Mutex<Vec<TrackedTicker>>>);

#[async_trait]
impl ForLoadingTrackedTickers for TrackedTickersMock {
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
    let application = TrackedTickersApplication::new(adapter.clone(), adapter);
    application
        .configure_ticker(TrackedTicker {
            ticker: " spy ".into(),
            active: true,
            historical_prices: true,
            option_snapshots: true,
        })
        .await
        .unwrap();
    assert_eq!(
        application.list_active_tickers().await.unwrap()[0].ticker,
        "SPY"
    );
}
