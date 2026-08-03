//! Market-data use cases.

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::{index_history::IndexHistory, live_price::LivePrice, market_history::MarketHistory},
    driven_ports::{
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_market_history::ForLoadingMarketHistory,
        for_obtaining_live_prices::ForObtainingLivePrices,
    },
    driving_ports::for_viewing_market_data::ForViewingMarketData,
};

/// Market-data application configured with its driven actor at runtime.
pub struct MarketDataApplication<MarketHistoryStore, IndexHistoryStore, LivePrices> {
    market_history_store: MarketHistoryStore,
    index_history_store: IndexHistoryStore,
    live_prices: LivePrices,
}

impl<MarketHistoryStore, IndexHistoryStore, LivePrices>
    MarketDataApplication<MarketHistoryStore, IndexHistoryStore, LivePrices>
{
    pub fn new(
        market_history_store: MarketHistoryStore,
        index_history_store: IndexHistoryStore,
        live_prices: LivePrices,
    ) -> Self {
        Self {
            market_history_store,
            index_history_store,
            live_prices,
        }
    }
}

#[async_trait]
impl<MarketHistoryStore, IndexHistoryStore, LivePrices> ForViewingMarketData
    for MarketDataApplication<MarketHistoryStore, IndexHistoryStore, LivePrices>
where
    MarketHistoryStore: ForLoadingMarketHistory,
    IndexHistoryStore: ForLoadingIndexHistory,
    LivePrices: ForObtainingLivePrices,
{
    async fn market_history(&self, ticker: &str) -> PortResult<MarketHistory> {
        let ticker = normalized_ticker(ticker)?;
        self.market_history_store.load_market_history(&ticker).await
    }

    async fn index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        let ticker = normalized_ticker(ticker)?;
        self.index_history_store.load_index_history(&ticker).await
    }

    async fn live_price(&self, ticker: &str) -> PortResult<LivePrice> {
        let ticker = normalized_ticker(ticker)?;
        self.live_prices.obtain_live_price(&ticker).await
    }
}

fn normalized_ticker(ticker: &str) -> PortResult<String> {
    let ticker = ticker.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || !ticker
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '^')
    {
        return Err(PortError::InvalidRequest(
            "ticker must contain only ASCII letters, digits, or '^'".to_string(),
        ));
    }
    Ok(ticker)
}
