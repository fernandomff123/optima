//! Business use cases, organized internally by capability.

pub mod index_history_migration;
pub mod interest_rates;
pub mod intraday_simulation;
pub mod market_data;
pub mod market_history_migration;
pub mod market_scheduling;
pub mod market_stream;
pub mod market_volatility;
pub mod option_chain_migration;
pub mod options;
pub mod portfolio;
pub mod portfolio_migration;
pub mod portfolio_valuation;
pub mod saved_strategies;
pub mod simulation;
pub mod strategy_migration;
pub mod synchronization;
pub mod tracked_ticker_migration;
pub mod tracked_tickers;
pub mod volatility_term_structure_migration;
pub mod yield_curve_migration;
