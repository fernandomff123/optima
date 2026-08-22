//! Market-volatility overview use case.

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        historical_volatility::{self, HistoricalVolatilityOverview},
        index_history::IndexHistory,
        market_volatility::{
            CalculatedVolatility, ImpliedVolatilityOverview, ImpliedVolatilityPoint,
            MarketVolatilityOverview, VolatilityIndexValue,
        },
    },
    driven_ports::{
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
    },
    driving_ports::for_viewing_volatility::{ForViewingVolatility, HistoricalVolatilityRequest},
};

const TERM_TICKERS: [&str; 4] = ["VIX9D", "VIX3M", "VIX6M", "VIX1Y"];

pub struct MarketVolatilityApplication<IndexHistoryStore, OptionDataStore, MarketHistoryStore> {
    index_history_store: IndexHistoryStore,
    option_data_store: OptionDataStore,
    market_history_store: MarketHistoryStore,
}

impl<IndexHistoryStore, OptionDataStore, MarketHistoryStore>
    MarketVolatilityApplication<IndexHistoryStore, OptionDataStore, MarketHistoryStore>
{
    pub fn new(
        index_history_store: IndexHistoryStore,
        option_data_store: OptionDataStore,
        market_history_store: MarketHistoryStore,
    ) -> Self {
        Self {
            index_history_store,
            option_data_store,
            market_history_store,
        }
    }
}

#[async_trait]
impl<IndexHistoryStore, OptionDataStore, MarketHistoryStore> ForViewingVolatility
    for MarketVolatilityApplication<IndexHistoryStore, OptionDataStore, MarketHistoryStore>
where
    IndexHistoryStore: ForLoadingIndexHistory,
    OptionDataStore: ForLoadingVolatilityTermStructures,
    MarketHistoryStore: ForLoadingMarketHistory,
{
    async fn volatility_overview(&self) -> PortResult<MarketVolatilityOverview> {
        let vix_history = self.index_history_store.load_index_history("VIX").await?;
        let as_of = vix_history
            .daily_prices
            .last()
            .map(|price| price.date)
            .ok_or_else(|| PortError::NotFound("no VIX session is stored".to_string()))?;
        let vix = index_value(&vix_history, as_of)
            .ok_or_else(|| PortError::NotFound("no VIX value is stored".to_string()))?;
        let vvix = self.index_value("VVIX", as_of).await?;
        let mut term_structure = Vec::new();
        for ticker in TERM_TICKERS {
            if let Some(value) = self.index_value(ticker, as_of).await? {
                term_structure.push(value);
            }
        }
        let session_end = as_of
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| PortError::InvalidRequest("invalid VIX session date".to_string()))?
            .and_utc();
        let spx_30_day = self
            .option_data_store
            .load_term_structure_at_or_before("SPX", session_end)
            .await?
            .filter(|term| term.snapshot_timestamp.date_naive() == as_of)
            .and_then(|term| {
                term.points
                    .iter()
                    .find(|point| (point.days - 30.0).abs() < 1e-9)
                    .map(|point| CalculatedVolatility {
                        ticker: term.ticker,
                        snapshot_timestamp: term.snapshot_timestamp,
                        volatility_percent: point.volatility,
                        difference_from_vix: point.volatility - vix.close,
                    })
            });
        Ok(MarketVolatilityOverview {
            as_of,
            vix,
            spx_30_day,
            vvix,
            term_structure,
        })
    }

    async fn historical_volatility(
        &self,
        request: HistoricalVolatilityRequest,
    ) -> PortResult<HistoricalVolatilityOverview> {
        let ticker = normalized_ticker(&request.ticker)?;
        validate_historical_volatility_request(&request)?;
        let history = self
            .market_history_store
            .load_market_history(&ticker)
            .await?;
        let mut horizons = request.horizons_sessions;
        horizons.sort_unstable();
        let mut overview =
            historical_volatility::analyze(&history, &horizons, request.series_limit);
        overview.ticker = ticker;
        Ok(overview)
    }

    async fn implied_volatility(&self, ticker: &str) -> PortResult<ImpliedVolatilityOverview> {
        let ticker = normalized_ticker(ticker)?;
        if let Some(reference_ticker) = implied_volatility_index(&ticker) {
            let history = self
                .index_history_store
                .load_index_history(reference_ticker)
                .await?;
            if !history.daily_prices.is_empty() {
                let start = history.daily_prices.len().saturating_sub(1_260);
                return Ok(ImpliedVolatilityOverview {
                    ticker,
                    reference_ticker: Some(reference_ticker.to_string()),
                    points: history.daily_prices[start..]
                        .iter()
                        .map(|point| ImpliedVolatilityPoint {
                            date: point.date,
                            volatility_percent: point.close,
                        })
                        .collect(),
                });
            }
        }
        let calculated = self
            .option_data_store
            .load_constant_maturity_volatility_history(&ticker, 30.0)
            .await?;
        let start = calculated.len().saturating_sub(1_260);
        Ok(ImpliedVolatilityOverview {
            ticker,
            reference_ticker: None,
            points: calculated[start..]
                .iter()
                .map(|point| ImpliedVolatilityPoint {
                    date: point.date,
                    volatility_percent: point.volatility,
                })
                .collect(),
        })
    }
}

fn validate_historical_volatility_request(request: &HistoricalVolatilityRequest) -> PortResult<()> {
    if request.horizons_sessions.is_empty() || request.horizons_sessions.len() > 6 {
        return Err(PortError::InvalidRequest(
            "horizons must contain between 1 and 6 values".to_string(),
        ));
    }
    if request
        .horizons_sessions
        .iter()
        .any(|value| !(2..=252).contains(value))
    {
        return Err(PortError::InvalidRequest(
            "each horizon must be between 2 and 252".to_string(),
        ));
    }
    let unique: std::collections::HashSet<_> = request.horizons_sessions.iter().collect();
    if unique.len() != request.horizons_sessions.len() {
        return Err(PortError::InvalidRequest(
            "horizons must not contain duplicates".to_string(),
        ));
    }
    if request.series_limit == 0 || request.series_limit > 1_260 {
        return Err(PortError::InvalidRequest(
            "limit must be between 1 and 1260".to_string(),
        ));
    }
    Ok(())
}

impl<IndexHistoryStore, OptionDataStore, MarketHistoryStore>
    MarketVolatilityApplication<IndexHistoryStore, OptionDataStore, MarketHistoryStore>
where
    IndexHistoryStore: ForLoadingIndexHistory,
{
    async fn index_value(
        &self,
        ticker: &str,
        as_of: chrono::NaiveDate,
    ) -> PortResult<Option<VolatilityIndexValue>> {
        let history = self.index_history_store.load_index_history(ticker).await?;
        Ok(index_value(&history, as_of))
    }
}

fn normalized_ticker(ticker: &str) -> PortResult<String> {
    let ticker = ticker.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || !ticker
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err(PortError::InvalidRequest("invalid ticker".to_string()));
    }
    Ok(ticker)
}

fn implied_volatility_index(ticker: &str) -> Option<&'static str> {
    match ticker {
        "AAPL" => Some("VXAPL"),
        "GOOGL" => Some("VXGOG"),
        "IBM" => Some("VXIBM"),
        "SPX" => Some("VIX"),
        _ => None,
    }
}

fn index_value(history: &IndexHistory, target: chrono::NaiveDate) -> Option<VolatilityIndexValue> {
    let mut prices = history
        .daily_prices
        .iter()
        .rev()
        .filter(|price| price.date <= target);
    let latest = prices.next()?;
    let daily_change_percent = prices.next().and_then(|previous| {
        (previous.close != 0.0).then(|| (latest.close / previous.close - 1.0) * 100.0)
    });
    Some(VolatilityIndexValue {
        ticker: history.ticker.clone(),
        date: latest.date,
        close: latest.close,
        daily_change_percent,
    })
}
