//! Market-data synchronization use cases.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::hexagon::{
    PortError, PortResult,
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_obtaining_volatility_indices::ForObtainingVolatilityIndices,
        for_obtaining_yield_curves::ForObtainingYieldCurves,
        for_storing_index_history::ForStoringIndexHistory,
        for_storing_market_history::ForStoringMarketHistory,
        for_storing_option_chains::ForStoringOptionChains,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
    driving_ports::for_synchronizing_market_data::{
        ForSynchronizingMarketData, SynchronizationFailure, SynchronizationReport,
        SynchronizeTrackedTickers, TrackedTickersSynchronizationReport,
    },
};

use super::options::build_term_structure;

/// Collaborators needed specifically for derived option analytics.
pub struct OptionAnalysisCollaborators<OptionChains, YieldCurves, TradingCalendar> {
    option_chains: OptionChains,
    yield_curves: YieldCurves,
    trading_calendar: TradingCalendar,
}

impl<OptionChains, YieldCurves, TradingCalendar>
    OptionAnalysisCollaborators<OptionChains, YieldCurves, TradingCalendar>
{
    pub fn new(
        option_chains: OptionChains,
        yield_curves: YieldCurves,
        trading_calendar: TradingCalendar,
    ) -> Self {
        Self {
            option_chains,
            yield_curves,
            trading_calendar,
        }
    }
}

/// Persistence participants grouped for constructor injection, while each
/// remains governed by its own business conversation.
pub struct SynchronizationStores<History, OptionChains, OptionData, Indices, Curves> {
    history: History,
    option_chains: OptionChains,
    option_data: OptionData,
    indices: Indices,
    curves: Curves,
}

impl<History, OptionChains, OptionData, Indices, Curves>
    SynchronizationStores<History, OptionChains, OptionData, Indices, Curves>
{
    pub fn new(
        history: History,
        option_chains: OptionChains,
        option_data: OptionData,
        indices: Indices,
        curves: Curves,
    ) -> Self {
        Self {
            history,
            option_chains,
            option_data,
            indices,
            curves,
        }
    }
}

/// Coordinates provider-neutral acquisition and persistence ports.
pub struct SynchronizationApplication<
    History,
    Options,
    Indices,
    Curves,
    HistoryStore,
    OptionChainStore,
    OptionStore,
    IndexStore,
    CurveStore,
    TrackedTickers,
    OptionChains,
    YieldCurves,
    TradingCalendar,
> {
    history: History,
    options: Options,
    indices: Indices,
    curves: Curves,
    stores:
        SynchronizationStores<HistoryStore, OptionChainStore, OptionStore, IndexStore, CurveStore>,
    tracked_tickers: TrackedTickers,
    option_analysis: OptionAnalysisCollaborators<OptionChains, YieldCurves, TradingCalendar>,
}

impl<
    History,
    Options,
    Indices,
    Curves,
    HistoryStore,
    OptionChainStore,
    OptionStore,
    IndexStore,
    CurveStore,
    TrackedTickers,
    OptionChains,
    YieldCurves,
    TradingCalendar,
>
    SynchronizationApplication<
        History,
        Options,
        Indices,
        Curves,
        HistoryStore,
        OptionChainStore,
        OptionStore,
        IndexStore,
        CurveStore,
        TrackedTickers,
        OptionChains,
        YieldCurves,
        TradingCalendar,
    >
{
    pub fn new(
        history: History,
        options: Options,
        indices: Indices,
        curves: Curves,
        stores: SynchronizationStores<
            HistoryStore,
            OptionChainStore,
            OptionStore,
            IndexStore,
            CurveStore,
        >,
        tracked_tickers: TrackedTickers,
        option_analysis: OptionAnalysisCollaborators<OptionChains, YieldCurves, TradingCalendar>,
    ) -> Self {
        Self {
            history,
            options,
            indices,
            curves,
            stores,
            tracked_tickers,
            option_analysis,
        }
    }
}

#[async_trait]
impl<
    History,
    Options,
    Indices,
    Curves,
    HistoryStore,
    OptionChainStore,
    OptionStore,
    IndexStore,
    CurveStore,
    TrackedTickers,
    OptionChains,
    YieldCurves,
    TradingCalendar,
> ForSynchronizingMarketData
    for SynchronizationApplication<
        History,
        Options,
        Indices,
        Curves,
        HistoryStore,
        OptionChainStore,
        OptionStore,
        IndexStore,
        CurveStore,
        TrackedTickers,
        OptionChains,
        YieldCurves,
        TradingCalendar,
    >
where
    History: ForObtainingMarketHistory,
    Options: ForObtainingOptionChains,
    Indices: ForObtainingVolatilityIndices,
    Curves: ForObtainingYieldCurves,
    HistoryStore: ForStoringMarketHistory,
    OptionChainStore: ForStoringOptionChains,
    OptionStore: ForStoringVolatilityTermStructures,
    IndexStore: ForStoringIndexHistory,
    CurveStore: ForStoringYieldCurves,
    TrackedTickers: ForLoadingTrackedTickers,
    OptionChains: ForLoadingOptionChains,
    YieldCurves: ForLoadingYieldCurves,
    TradingCalendar: ForConsultingTradingCalendar,
{
    async fn synchronize_tracked_tickers(
        &self,
        request: SynchronizeTrackedTickers,
    ) -> PortResult<TrackedTickersSynchronizationReport> {
        let tickers = self.tracked_tickers.load_active_tickers().await?;
        let mut report = TrackedTickersSynchronizationReport {
            tickers: tickers.len(),
            items_obtained: 0,
            items_stored: 0,
            failures: Vec::new(),
        };
        for tracked in tickers {
            if tracked.historical_prices {
                match self
                    .synchronize_market_history(&tracked.ticker, request.since)
                    .await
                {
                    Ok(result) => {
                        report.items_obtained += result.items_obtained;
                        report.items_stored += result.items_stored;
                    }
                    Err(error) => report.failures.push(SynchronizationFailure {
                        ticker: tracked.ticker.clone(),
                        operation: "market_history".to_string(),
                        error: error.to_string(),
                    }),
                }
            }
            if tracked.option_snapshots {
                let chain_synchronized = match self
                    .synchronize_option_chain(&tracked.ticker, request.market_close)
                    .await
                {
                    Ok(result) => {
                        report.items_obtained += result.items_obtained;
                        report.items_stored += result.items_stored;
                        true
                    }
                    Err(error) => {
                        report.failures.push(SynchronizationFailure {
                            ticker: tracked.ticker.clone(),
                            operation: "option_chain".to_string(),
                            error: error.to_string(),
                        });
                        false
                    }
                };
                if chain_synchronized {
                    match self.synchronize_term_structure(&tracked.ticker).await {
                        Ok(result) => {
                            report.items_obtained += result.items_obtained;
                            report.items_stored += result.items_stored;
                        }
                        Err(error) => report.failures.push(SynchronizationFailure {
                            ticker: tracked.ticker,
                            operation: "term_structure".to_string(),
                            error: error.to_string(),
                        }),
                    }
                }
            }
        }
        Ok(report)
    }

    async fn synchronize_market_history(
        &self,
        ticker: &str,
        since: NaiveDate,
    ) -> PortResult<SynchronizationReport> {
        let ticker = normalized_ticker(ticker)?;
        let history = self.history.obtain_market_history(&ticker, since).await?;
        let items_obtained =
            history.daily_quotes.len() + history.dividends.len() + history.splits.len();
        let items_stored = self.stores.history.store_market_history(&history).await?;
        Ok(SynchronizationReport {
            items_obtained,
            items_stored,
        })
    }

    async fn synchronize_option_chain(
        &self,
        ticker: &str,
        market_close: DateTime<Utc>,
    ) -> PortResult<SynchronizationReport> {
        let ticker = normalized_ticker(ticker)?;
        let snapshot = self.options.obtain_option_chain(&ticker).await?;
        let items_obtained = snapshot.contratos.len();
        let items_stored = self
            .stores
            .option_chains
            .store_option_chain(&snapshot, market_close)
            .await?;
        Ok(SynchronizationReport {
            items_obtained,
            items_stored,
        })
    }

    async fn synchronize_term_structure(&self, ticker: &str) -> PortResult<SynchronizationReport> {
        let ticker = normalized_ticker(ticker)?;
        let term_structure = build_term_structure(
            &self.option_analysis.option_chains,
            &self.option_analysis.yield_curves,
            &self.option_analysis.trading_calendar,
            &ticker,
        )
        .await?;
        let items_obtained = term_structure.points.len();
        let items_stored = self
            .stores
            .option_data
            .store_term_structure(&term_structure)
            .await?;
        Ok(SynchronizationReport {
            items_obtained,
            items_stored,
        })
    }

    async fn synchronize_volatility_index(
        &self,
        ticker: &str,
    ) -> PortResult<SynchronizationReport> {
        let ticker = normalized_ticker(ticker)?;
        let history = self.indices.obtain_volatility_index(&ticker).await?;
        let items_obtained = history.daily_prices.len();
        let items_stored = self.stores.indices.store_index_history(&history).await?;
        Ok(SynchronizationReport {
            items_obtained,
            items_stored,
        })
    }

    async fn synchronize_yield_curves(&self, year: i32) -> PortResult<SynchronizationReport> {
        if !(1900..=2100).contains(&year) {
            return Err(PortError::InvalidRequest(format!("invalid year: {year}")));
        }
        let curves = self.curves.obtain_yield_curves(year).await?;
        let items_obtained = curves.len();
        let items_stored = self.stores.curves.store_yield_curves(&curves).await?;
        Ok(SynchronizationReport {
            items_obtained,
            items_stored,
        })
    }
}

fn normalized_ticker(ticker: &str) -> PortResult<String> {
    let ticker = ticker.trim();
    if ticker.is_empty() {
        return Err(PortError::InvalidRequest(
            "ticker must not be empty".to_string(),
        ));
    }
    Ok(ticker.to_ascii_uppercase())
}
