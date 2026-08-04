use std::sync::Mutex;

use async_trait::async_trait;
use hexagonal_backend::hexagon::{
    PortResult,
    application::market_data::MarketDataApplication,
    domain::{index_history::IndexHistory, live_price::LivePrice, market_history::MarketHistory},
    driven_ports::{
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_market_history::ForLoadingMarketHistory,
        for_obtaining_live_prices::ForObtainingLivePrices,
    },
    driving_ports::for_viewing_market_data::ForViewingMarketData,
};

#[derive(Default)]
struct MarketHistoryStoreMock {
    requested_tickers: Mutex<Vec<String>>,
}

struct LivePricesMock;

struct IndexHistoryStoreMock;

#[async_trait]
impl ForLoadingIndexHistory for IndexHistoryStoreMock {
    async fn load_index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        Ok(IndexHistory {
            ticker: ticker.to_string(),
            daily_prices: Vec::new(),
        })
    }
}

#[async_trait]
impl ForObtainingLivePrices for LivePricesMock {
    async fn obtain_live_price(&self, ticker: &str) -> PortResult<LivePrice> {
        Ok(LivePrice {
            ticker: ticker.to_string(),
            price: 100.0,
            market_time: 0,
            currency: "USD".to_string(),
            exchange: "TEST".to_string(),
            regular_session: true,
            change: 0.0,
            change_percent: 0.0,
            day_volume: 0,
        })
    }
}

#[async_trait]
impl ForLoadingMarketHistory for MarketHistoryStoreMock {
    async fn load_market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        self.requested_tickers
            .lock()
            .expect("test mutex must not be poisoned")
            .push(ticker.to_string());
        Ok(MarketHistory {
            ticker: ticker.to_string(),
            currency: None,
            exchange_timezone: None,
            daily_quotes: Vec::new(),
            dividends: Vec::new(),
            splits: Vec::new(),
        })
    }
}

#[tokio::test]
async fn test_drives_the_app_while_a_mock_plays_the_driven_actor() {
    let app = MarketDataApplication::new(
        MarketHistoryStoreMock::default(),
        IndexHistoryStoreMock,
        LivePricesMock,
    );

    let history = app.market_history(" aapl ").await.unwrap();

    assert_eq!(history.ticker, "AAPL");
}

#[tokio::test]
async fn invalid_input_is_rejected_before_calling_the_driven_port() {
    let app = MarketDataApplication::new(
        MarketHistoryStoreMock::default(),
        IndexHistoryStoreMock,
        LivePricesMock,
    );

    let error = app.market_history("AAPL/USD").await.unwrap_err();

    assert!(error.to_string().contains("ticker"));
}

#[tokio::test]
async fn same_driving_port_exposes_live_and_historical_market_data() {
    let app = MarketDataApplication::new(
        MarketHistoryStoreMock::default(),
        IndexHistoryStoreMock,
        LivePricesMock,
    );

    let price = app.live_price(" spy ").await.unwrap();

    assert_eq!(price.ticker, "SPY");
}

#[tokio::test]
async fn index_history_has_its_own_mocked_driven_port() {
    let app = MarketDataApplication::new(
        MarketHistoryStoreMock::default(),
        IndexHistoryStoreMock,
        LivePricesMock,
    );

    let history = app.index_history(" vix ").await.unwrap();

    assert_eq!(history.ticker, "VIX");
}
