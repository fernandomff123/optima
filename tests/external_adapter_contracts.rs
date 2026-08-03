//! Compile-time contract checks for production driven adapters.
//!
//! Recorded DTO/parser tests live beside each adapter. These checks make the
//! architectural promise explicit: concrete providers satisfy application-owned
//! ports without exposing provider types to the hexagon.

use polars_options::{
    driven_adapters::{
        cboe::{CboeOptionChainsAdapter, CboeVolatilityIndicesAdapter},
        exchange_calendar::ExchangeTradingCalendarAdapter,
        sqlite::market_data::SqliteMarketDataAdapter,
        sqlite::saved_strategies::SqliteSavedStrategiesAdapter,
        sqlite::tracked_tickers::SqliteTrackedTickersAdapter,
        treasury::TreasuryYieldCurvesAdapter,
        yahoo::{YahooLivePricesAdapter, YahooMarketHistoryAdapter},
    },
    hexagon::driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_strategies::ForLoadingStrategies,
        for_loading_tracked_tickers::ForLoadingTrackedTickers,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_obtaining_volatility_indices::ForObtainingVolatilityIndices,
        for_obtaining_yield_curves::ForObtainingYieldCurves,
        for_storing_market_data::ForStoringMarketData,
        for_storing_strategies::ForStoringStrategies,
        for_storing_tracked_tickers::ForStoringTrackedTickers,
    },
};

fn implements_market_history_port<T: ForObtainingMarketHistory>() {}
fn implements_live_prices_port<T: ForObtainingLivePrices>() {}
fn implements_option_chains_port<T: ForObtainingOptionChains>() {}
fn implements_volatility_indices_port<T: ForObtainingVolatilityIndices>() {}
fn implements_yield_curves_port<T: ForObtainingYieldCurves>() {}
fn implements_trading_calendar_port<T: ForConsultingTradingCalendar>() {}
fn implements_market_data_store_port<T: ForStoringMarketData>() {}
fn implements_strategy_loader_port<T: ForLoadingStrategies>() {}
fn implements_strategy_store_port<T: ForStoringStrategies>() {}
fn implements_tracked_ticker_loader_port<T: ForLoadingTrackedTickers>() {}
fn implements_tracked_ticker_store_port<T: ForStoringTrackedTickers>() {}

#[test]
fn external_adapters_implement_provider_neutral_ports() {
    implements_market_history_port::<YahooMarketHistoryAdapter>();
    implements_live_prices_port::<YahooLivePricesAdapter>();
    implements_option_chains_port::<CboeOptionChainsAdapter>();
    implements_volatility_indices_port::<CboeVolatilityIndicesAdapter>();
    implements_yield_curves_port::<TreasuryYieldCurvesAdapter>();
    implements_trading_calendar_port::<ExchangeTradingCalendarAdapter>();
    implements_market_data_store_port::<SqliteMarketDataAdapter>();
    implements_strategy_loader_port::<SqliteSavedStrategiesAdapter>();
    implements_strategy_store_port::<SqliteSavedStrategiesAdapter>();
    implements_tracked_ticker_loader_port::<SqliteTrackedTickersAdapter>();
    implements_tracked_ticker_store_port::<SqliteTrackedTickersAdapter>();
}
