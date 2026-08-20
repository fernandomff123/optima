//! Compile-time contract checks for production driven adapters.
//!
//! Recorded DTO/parser tests live beside each adapter. These checks make the
//! architectural promise explicit: concrete providers satisfy application-owned
//! ports without exposing provider types to the hexagon.

use hexagonal_backend::{
    driven_adapters::{
        cboe::{CboeOptionChainsAdapter, CboeVolatilityIndicesAdapter},
        duckdb::{
            index_history::DuckDbIndexHistoryAdapter, market_history::DuckDbMarketHistoryAdapter,
            option_chains::DuckDbOptionChainsAdapter, portfolio::DuckDbPortfolioAdapter,
            saved_strategies::DuckDbSavedStrategiesAdapter,
            tracked_tickers::DuckDbTrackedTickersAdapter,
            volatility_term_structures::DuckDbVolatilityTermStructuresAdapter,
            yield_curves::DuckDbYieldCurvesAdapter,
        },
        exchange_calendar::ExchangeTradingCalendarAdapter,
        sqlite::portfolio::SqlitePortfolioAdapter,
        sqlite::saved_strategies::SqliteSavedStrategiesAdapter,
        sqlite::tracked_tickers::SqliteTrackedTickersAdapter,
        sqlite::{
            index_history_port::SqliteIndexHistoryAdapter,
            market_history::SqliteMarketHistoryAdapter, option_data::SqliteOptionDataAdapter,
            yield_curves_port::SqliteYieldCurvesAdapter,
        },
        treasury::TreasuryYieldCurvesAdapter,
        yahoo::{
            YahooLivePricesAdapter, YahooMarketHistoryAdapter, YahooUnderlyingResolverAdapter,
        },
    },
    hexagon::driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_portfolios::ForLoadingPortfolios,
        for_loading_reference_prices::ForLoadingReferencePrices,
        for_loading_strategies::ForLoadingStrategies,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_obtaining_volatility_indices::ForObtainingVolatilityIndices,
        for_obtaining_yield_curves::ForObtainingYieldCurves,
        for_resolving_underlying_symbols::ForResolvingUnderlyingSymbols,
        for_storing_index_history::ForStoringIndexHistory,
        for_storing_market_history::ForStoringMarketHistory,
        for_storing_option_chains::ForStoringOptionChains,
        for_storing_portfolios::ForStoringPortfolios, for_storing_strategies::ForStoringStrategies,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
};

fn implements_market_history_port<T: ForObtainingMarketHistory>() {}
fn implements_live_prices_port<T: ForObtainingLivePrices>() {}
fn implements_option_chains_port<T: ForObtainingOptionChains>() {}
fn implements_volatility_indices_port<T: ForObtainingVolatilityIndices>() {}
fn implements_yield_curves_port<T: ForObtainingYieldCurves>() {}
fn implements_trading_calendar_port<T: ForConsultingTradingCalendar>() {}
fn implements_market_history_store_port<T: ForStoringMarketHistory>() {}
fn implements_market_history_loader_port<T: ForLoadingMarketHistory>() {}
fn implements_option_data_store_port<T: ForStoringVolatilityTermStructures>() {}
fn implements_term_structure_loader_port<T: ForLoadingVolatilityTermStructures>() {}
fn implements_reference_price_loader_port<T: ForLoadingReferencePrices>() {}
fn implements_option_chain_loader_port<T: ForLoadingOptionChains>() {}
fn implements_option_chain_store_port<T: ForStoringOptionChains>() {}
fn implements_index_history_store_port<T: ForStoringIndexHistory>() {}
fn implements_index_history_loader_port<T: ForLoadingIndexHistory>() {}
fn implements_yield_curves_store_port<T: ForStoringYieldCurves>() {}
fn implements_yield_curves_loader_port<T: ForLoadingYieldCurves>() {}
fn implements_strategy_loader_port<T: ForLoadingStrategies>() {}
fn implements_strategy_store_port<T: ForStoringStrategies>() {}
fn implements_portfolio_loader_port<T: ForLoadingPortfolios>() {}
fn implements_portfolio_store_port<T: ForStoringPortfolios>() {}
fn implements_tracked_ticker_loader_port<T: ForLoadingTrackedTickers>() {}
fn implements_tracked_ticker_store_port<T: ForStoringTrackedTickers>() {}
fn implements_underlying_resolver_port<T: ForResolvingUnderlyingSymbols>() {}

#[test]
fn external_adapters_implement_provider_neutral_ports() {
    implements_market_history_port::<YahooMarketHistoryAdapter>();
    implements_live_prices_port::<YahooLivePricesAdapter>();
    implements_underlying_resolver_port::<YahooUnderlyingResolverAdapter>();
    implements_option_chains_port::<CboeOptionChainsAdapter>();
    implements_volatility_indices_port::<CboeVolatilityIndicesAdapter>();
    implements_yield_curves_port::<TreasuryYieldCurvesAdapter>();
    implements_trading_calendar_port::<ExchangeTradingCalendarAdapter>();
    implements_market_history_store_port::<SqliteMarketHistoryAdapter>();
    implements_market_history_loader_port::<SqliteMarketHistoryAdapter>();
    implements_market_history_store_port::<DuckDbMarketHistoryAdapter>();
    implements_market_history_loader_port::<DuckDbMarketHistoryAdapter>();
    implements_option_data_store_port::<SqliteOptionDataAdapter>();
    implements_term_structure_loader_port::<SqliteOptionDataAdapter>();
    implements_reference_price_loader_port::<SqliteOptionDataAdapter>();
    implements_option_data_store_port::<DuckDbVolatilityTermStructuresAdapter>();
    implements_term_structure_loader_port::<DuckDbVolatilityTermStructuresAdapter>();
    implements_reference_price_loader_port::<DuckDbVolatilityTermStructuresAdapter>();
    implements_option_chain_loader_port::<SqliteOptionDataAdapter>();
    implements_option_chain_store_port::<SqliteOptionDataAdapter>();
    implements_option_chain_loader_port::<DuckDbOptionChainsAdapter>();
    implements_option_chain_store_port::<DuckDbOptionChainsAdapter>();
    implements_index_history_store_port::<SqliteIndexHistoryAdapter>();
    implements_index_history_loader_port::<SqliteIndexHistoryAdapter>();
    implements_index_history_store_port::<DuckDbIndexHistoryAdapter>();
    implements_index_history_loader_port::<DuckDbIndexHistoryAdapter>();
    implements_yield_curves_store_port::<SqliteYieldCurvesAdapter>();
    implements_yield_curves_loader_port::<SqliteYieldCurvesAdapter>();
    implements_yield_curves_store_port::<DuckDbYieldCurvesAdapter>();
    implements_yield_curves_loader_port::<DuckDbYieldCurvesAdapter>();
    implements_strategy_loader_port::<SqliteSavedStrategiesAdapter>();
    implements_strategy_store_port::<SqliteSavedStrategiesAdapter>();
    implements_strategy_loader_port::<DuckDbSavedStrategiesAdapter>();
    implements_strategy_store_port::<DuckDbSavedStrategiesAdapter>();
    implements_portfolio_loader_port::<DuckDbPortfolioAdapter>();
    implements_portfolio_store_port::<DuckDbPortfolioAdapter>();
    implements_portfolio_loader_port::<SqlitePortfolioAdapter>();
    implements_portfolio_store_port::<SqlitePortfolioAdapter>();
    implements_tracked_ticker_loader_port::<SqliteTrackedTickersAdapter>();
    implements_tracked_ticker_store_port::<SqliteTrackedTickersAdapter>();
    implements_tracked_ticker_loader_port::<DuckDbTrackedTickersAdapter>();
    implements_tracked_ticker_store_port::<DuckDbTrackedTickersAdapter>();
}
