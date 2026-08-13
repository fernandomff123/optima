use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use hexagonal_backend::hexagon::{
    PortError, PortResult,
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
    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone());
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
    let application = TrackedTickersApplication::new(adapter.clone(), adapter);
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
