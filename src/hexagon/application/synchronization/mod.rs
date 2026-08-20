//! Market-data synchronization use cases.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeSet;

use crate::hexagon::domain::options::OptionIngestionWarning;
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
        for_resolving_option_contract_specifications::{
            ForResolvingOptionContractSpecifications, OptionContractIdentity,
        },
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

pub struct OptionSnapshotEnrichment<ContractSpecifications> {
    contract_specifications: ContractSpecifications,
}

impl<ContractSpecifications> OptionSnapshotEnrichment<ContractSpecifications> {
    pub fn new(contract_specifications: ContractSpecifications) -> Self {
        Self {
            contract_specifications,
        }
    }
}

pub struct SynchronizationSources<History, Options, Indices, Curves> {
    history: History,
    options: Options,
    indices: Indices,
    curves: Curves,
}

impl<History, Options, Indices, Curves> SynchronizationSources<History, Options, Indices, Curves> {
    pub fn new(history: History, options: Options, indices: Indices, curves: Curves) -> Self {
        Self {
            history,
            options,
            indices,
            curves,
        }
    }
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
    ContractSpecifications,
> {
    sources: SynchronizationSources<History, Options, Indices, Curves>,
    stores:
        SynchronizationStores<HistoryStore, OptionChainStore, OptionStore, IndexStore, CurveStore>,
    tracked_tickers: TrackedTickers,
    option_analysis: OptionAnalysisCollaborators<OptionChains, YieldCurves, TradingCalendar>,
    option_snapshot_enrichment: OptionSnapshotEnrichment<ContractSpecifications>,
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
    ContractSpecifications,
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
        ContractSpecifications,
    >
{
    pub fn new(
        sources: SynchronizationSources<History, Options, Indices, Curves>,
        stores: SynchronizationStores<
            HistoryStore,
            OptionChainStore,
            OptionStore,
            IndexStore,
            CurveStore,
        >,
        tracked_tickers: TrackedTickers,
        option_analysis: OptionAnalysisCollaborators<OptionChains, YieldCurves, TradingCalendar>,
        option_snapshot_enrichment: OptionSnapshotEnrichment<ContractSpecifications>,
    ) -> Self {
        Self {
            sources,
            stores,
            tracked_tickers,
            option_analysis,
            option_snapshot_enrichment,
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
    ContractSpecifications,
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
        ContractSpecifications,
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
    ContractSpecifications: ForResolvingOptionContractSpecifications,
{
    async fn synchronize_tracked_tickers(
        &self,
        request: SynchronizeTrackedTickers,
    ) -> PortResult<TrackedTickersSynchronizationReport> {
        let tickers = self.tracked_tickers.load_refresh_eligible_tickers().await?;
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
        let history = self
            .sources
            .history
            .obtain_market_history(&ticker, since)
            .await?;
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
        let mut snapshot = self.sources.options.obtain_option_chain(&ticker).await?;
        let mut unresolved_roots = BTreeSet::new();
        for chain in &mut snapshot.chains {
            for contract in &mut chain.contratos {
                let specification = self
                    .option_snapshot_enrichment
                    .contract_specifications
                    .resolve_option_contract_specification(OptionContractIdentity {
                        root: &chain.root,
                        occ_symbol: &contract.occ_symbol,
                    })
                    .await?;
                if specification.is_none() {
                    unresolved_roots.insert(chain.root.clone());
                }
                contract.contract_specification = specification;
            }
        }
        let evidenced_currency =
            single_evidenced_currency(snapshot.chains.iter().flat_map(|chain| {
                chain.contratos.iter().map(|contract| {
                    contract
                        .contract_specification
                        .as_ref()
                        .map(|specification| specification.currency.as_str())
                })
            }));
        snapshot.contratos = snapshot
            .chains
            .iter()
            .flat_map(|chain| chain.contratos.iter().cloned())
            .collect();
        if unresolved_roots.is_empty()
            && let (Some(underlying), Some(currency)) =
                (&mut snapshot.underlying_price, evidenced_currency)
        {
            underlying.currency = Some(currency);
        }
        for warning in unresolved_roots
            .into_iter()
            .map(|root| OptionIngestionWarning::ContractSpecificationUnavailable { root })
        {
            snapshot.ingestion_diagnostics.record_warning(warning);
        }
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
        let history = self
            .sources
            .indices
            .obtain_volatility_index(&ticker)
            .await?;
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
        let curves = self.sources.curves.obtain_yield_curves(year).await?;
        let items_obtained = curves.len();
        let items_stored = self.stores.curves.store_yield_curves(&curves).await?;
        Ok(SynchronizationReport {
            items_obtained,
            items_stored,
        })
    }
}

fn single_evidenced_currency<'a>(
    specifications: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<String> {
    let mut currencies = BTreeSet::new();
    for currency in specifications {
        currencies.insert(currency?);
    }
    if currencies.len() == 1 {
        currencies.into_iter().next().map(str::to_owned)
    } else {
        None
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

#[cfg(test)]
mod tests {
    use super::single_evidenced_currency;

    #[test]
    fn underlying_currency_requires_one_currency_on_every_contract() {
        assert_eq!(
            single_evidenced_currency([Some("USD"), Some("USD"), Some("USD")]),
            Some("USD".to_string())
        );
        assert_eq!(
            single_evidenced_currency([Some("USD"), Some("EUR"), Some("USD")]),
            None
        );
        assert_eq!(
            single_evidenced_currency([Some("EUR"), Some("USD"), Some("USD")]),
            None
        );
        assert_eq!(
            single_evidenced_currency([Some("USD"), None, Some("USD")]),
            None
        );
    }
}
