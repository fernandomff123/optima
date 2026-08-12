//! Composition root that knows and connects all concrete participants.
//!
//! No application module constructs an adapter. This outer component is the
//! only place where provider and database choices are combined.

use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;

use crate::{
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
        sqlite::{
            index_history_port::SqliteIndexHistoryAdapter,
            market_history::SqliteMarketHistoryAdapter, option_data::SqliteOptionDataAdapter,
            portfolio::SqlitePortfolioAdapter, saved_strategies::SqliteSavedStrategiesAdapter,
            tracked_tickers::SqliteTrackedTickersAdapter,
            yield_curves_port::SqliteYieldCurvesAdapter,
        },
        treasury::TreasuryYieldCurvesAdapter,
        yahoo::{YahooLivePricesAdapter, YahooMarketHistoryAdapter},
    },
    hexagon::application::{
        index_history_migration::IndexHistoryMigrationApplication,
        interest_rates::InterestRatesApplication,
        intraday_simulation::IntradaySimulationApplication,
        market_data::MarketDataApplication,
        market_history_migration::MarketHistoryMigrationApplication,
        market_scheduling::MarketSchedulingApplication,
        market_stream::MarketStreamApplication,
        market_volatility::MarketVolatilityApplication,
        option_chain_migration::OptionChainMigrationApplication,
        options::OptionsApplication,
        portfolio::PortfolioApplication,
        portfolio_migration::PortfolioMigrationApplication,
        portfolio_valuation::{PortfolioValuationApplication, PricingCollaborators},
        saved_strategies::SavedStrategiesApplication,
        sector_performance::SectorPerformanceApplication,
        simulation::SimulationApplication,
        strategy_migration::StrategyMigrationApplication,
        synchronization::{
            OptionAnalysisCollaborators, SynchronizationApplication, SynchronizationStores,
        },
        tracked_ticker_migration::TrackedTickerMigrationApplication,
        tracked_tickers::TrackedTickersApplication,
        volatility_term_structure_migration::VolatilityTermStructureMigrationApplication,
        yield_curve_migration::YieldCurveMigrationApplication,
    },
};

pub type ConfiguredMarketData = MarketDataApplication<
    DuckDbMarketHistoryAdapter,
    DuckDbIndexHistoryAdapter,
    YahooLivePricesAdapter,
>;
pub type ConfiguredMarketStream = MarketStreamApplication<YahooLivePricesAdapter>;
pub type ConfiguredMarketScheduling = MarketSchedulingApplication<ExchangeTradingCalendarAdapter>;
pub type ConfiguredOptions = OptionsApplication<
    DuckDbOptionChainsAdapter,
    DuckDbVolatilityTermStructuresAdapter,
    DuckDbYieldCurvesAdapter,
    ExchangeTradingCalendarAdapter,
>;
pub type ConfiguredInterestRates = InterestRatesApplication<DuckDbYieldCurvesAdapter>;
pub type ConfiguredMarketVolatility = MarketVolatilityApplication<
    DuckDbIndexHistoryAdapter,
    DuckDbVolatilityTermStructuresAdapter,
    DuckDbMarketHistoryAdapter,
>;
pub type ConfiguredIntradaySimulation = IntradaySimulationApplication<
    CboeOptionChainsAdapter,
    YahooLivePricesAdapter,
    ExchangeTradingCalendarAdapter,
>;
pub type ConfiguredPortfolios =
    PortfolioApplication<DuckDbPortfolioAdapter, DuckDbPortfolioAdapter>;
pub type ConfiguredPortfolioValuation = PortfolioValuationApplication<
    DuckDbPortfolioAdapter,
    PricingCollaborators<
        ExchangeTradingCalendarAdapter,
        YahooLivePricesAdapter,
        DuckDbMarketHistoryAdapter,
        CboeOptionChainsAdapter,
        DuckDbOptionChainsAdapter,
    >,
>;
pub type ConfiguredSavedStrategies =
    SavedStrategiesApplication<DuckDbSavedStrategiesAdapter, DuckDbSavedStrategiesAdapter>;
pub type ConfiguredTrackedTickers =
    TrackedTickersApplication<DuckDbTrackedTickersAdapter, DuckDbTrackedTickersAdapter>;
pub type ConfiguredSectorPerformance =
    SectorPerformanceApplication<DuckDbMarketHistoryAdapter, ExchangeTradingCalendarAdapter>;
pub type ConfiguredSynchronization = SynchronizationApplication<
    YahooMarketHistoryAdapter,
    CboeOptionChainsAdapter,
    CboeVolatilityIndicesAdapter,
    TreasuryYieldCurvesAdapter,
    DuckDbMarketHistoryAdapter,
    DuckDbOptionChainsAdapter,
    DuckDbVolatilityTermStructuresAdapter,
    DuckDbIndexHistoryAdapter,
    DuckDbYieldCurvesAdapter,
    DuckDbTrackedTickersAdapter,
    DuckDbOptionChainsAdapter,
    DuckDbYieldCurvesAdapter,
    ExchangeTradingCalendarAdapter,
>;
pub type ConfiguredOptionChainMigration =
    OptionChainMigrationApplication<SqliteOptionDataAdapter, DuckDbOptionChainsAdapter>;
pub type ConfiguredMarketHistoryMigration =
    MarketHistoryMigrationApplication<SqliteMarketHistoryAdapter, DuckDbMarketHistoryAdapter>;
pub type ConfiguredIndexHistoryMigration =
    IndexHistoryMigrationApplication<SqliteIndexHistoryAdapter, DuckDbIndexHistoryAdapter>;
pub type ConfiguredYieldCurveMigration =
    YieldCurveMigrationApplication<SqliteYieldCurvesAdapter, DuckDbYieldCurvesAdapter>;
pub type ConfiguredVolatilityTermStructureMigration = VolatilityTermStructureMigrationApplication<
    SqliteOptionDataAdapter,
    DuckDbVolatilityTermStructuresAdapter,
>;
pub type ConfiguredTrackedTickerMigration =
    TrackedTickerMigrationApplication<SqliteTrackedTickersAdapter, DuckDbTrackedTickersAdapter>;
pub type ConfiguredPortfolioMigration =
    PortfolioMigrationApplication<SqlitePortfolioAdapter, DuckDbPortfolioAdapter>;
pub type ConfiguredStrategyMigration =
    StrategyMigrationApplication<SqliteSavedStrategiesAdapter, DuckDbSavedStrategiesAdapter>;

/// Fully wired single hexagon, ready to be handed to driving adapters.
pub struct ConfiguredApplication {
    pub market_data: ConfiguredMarketData,
    pub market_stream: ConfiguredMarketStream,
    pub market_scheduling: ConfiguredMarketScheduling,
    pub interest_rates: ConfiguredInterestRates,
    pub market_volatility: ConfiguredMarketVolatility,
    pub intraday_simulation: ConfiguredIntradaySimulation,
    pub options: ConfiguredOptions,
    pub portfolios: ConfiguredPortfolios,
    pub portfolio_valuation: ConfiguredPortfolioValuation,
    pub saved_strategies: ConfiguredSavedStrategies,
    pub tracked_tickers: ConfiguredTrackedTickers,
    pub sector_performance: ConfiguredSectorPerformance,
    pub simulation: SimulationApplication,
    pub synchronization: ConfiguredSynchronization,
}

/// Prepares every schema owned by a SQLite driven adapter.
///
/// Schema setup belongs to the composition/bootstrap edge. Applications and
/// domain objects never invoke migrations.
pub async fn initialize_storage(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    use crate::driven_adapters::sqlite::{
        index_history, market_history, migrations, option_snapshots, portfolio, saved_strategies,
        tracked_tickers, volatility_term_structures, yield_curves,
    };

    tracked_tickers::initialize(pool).await?;
    tracked_tickers::seed_defaults(pool).await?;
    migrations::remove_research_storage(pool).await?;
    market_history::initialize(pool).await?;
    index_history::initialize(pool).await?;
    option_snapshots::initialize(pool).await?;
    volatility_term_structures::initialize(pool).await?;
    yield_curves::initialize(pool).await?;
    portfolio::initialize(pool).await?;
    saved_strategies::initialize(pool).await?;
    Ok(())
}

/// Prepares the analytical schemas selected by the production configurator.
pub async fn initialize_analytical_storage() -> crate::hexagon::PortResult<()> {
    let path = "data/market_data.duckdb";
    DuckDbOptionChainsAdapter::new(path).initialize().await?;
    DuckDbMarketHistoryAdapter::new(path).initialize().await?;
    DuckDbIndexHistoryAdapter::new(path).initialize().await?;
    DuckDbYieldCurvesAdapter::new(path).initialize().await?;
    DuckDbVolatilityTermStructuresAdapter::new(path)
        .initialize()
        .await?;
    DuckDbTrackedTickersAdapter::new(path).initialize().await?;
    DuckDbPortfolioAdapter::new(path).initialize().await?;
    DuckDbSavedStrategiesAdapter::new(path).initialize().await
}

/// Selects concrete production adapters and injects them through constructors.
pub fn configure() -> ConfiguredApplication {
    let portfolio_adapter = DuckDbPortfolioAdapter::new("data/market_data.duckdb");
    let strategies_adapter = DuckDbSavedStrategiesAdapter::new("data/market_data.duckdb");
    let tracked_tickers_adapter = DuckDbTrackedTickersAdapter::new("data/market_data.duckdb");
    ConfiguredApplication {
        market_data: MarketDataApplication::new(
            DuckDbMarketHistoryAdapter::new("data/market_data.duckdb"),
            DuckDbIndexHistoryAdapter::new("data/market_data.duckdb"),
            YahooLivePricesAdapter,
        ),
        market_stream: configure_market_stream(),
        market_scheduling: MarketSchedulingApplication::new(ExchangeTradingCalendarAdapter),
        interest_rates: InterestRatesApplication::new(DuckDbYieldCurvesAdapter::new(
            "data/market_data.duckdb",
        )),
        market_volatility: MarketVolatilityApplication::new(
            DuckDbIndexHistoryAdapter::new("data/market_data.duckdb"),
            DuckDbVolatilityTermStructuresAdapter::new("data/market_data.duckdb"),
            DuckDbMarketHistoryAdapter::new("data/market_data.duckdb"),
        ),
        intraday_simulation: IntradaySimulationApplication::new(
            CboeOptionChainsAdapter,
            YahooLivePricesAdapter,
            ExchangeTradingCalendarAdapter,
        ),
        options: OptionsApplication::new(
            DuckDbOptionChainsAdapter::new("data/market_data.duckdb"),
            DuckDbVolatilityTermStructuresAdapter::new("data/market_data.duckdb"),
            DuckDbYieldCurvesAdapter::new("data/market_data.duckdb"),
            ExchangeTradingCalendarAdapter,
        ),
        portfolios: PortfolioApplication::new(portfolio_adapter.clone(), portfolio_adapter),
        portfolio_valuation: PortfolioValuationApplication::new(
            DuckDbPortfolioAdapter::new("data/market_data.duckdb"),
            PricingCollaborators::new(
                ExchangeTradingCalendarAdapter,
                YahooLivePricesAdapter,
                DuckDbMarketHistoryAdapter::new("data/market_data.duckdb"),
                CboeOptionChainsAdapter,
                DuckDbOptionChainsAdapter::new("data/market_data.duckdb"),
            ),
        ),
        saved_strategies: SavedStrategiesApplication::new(
            strategies_adapter.clone(),
            strategies_adapter,
        ),
        tracked_tickers: TrackedTickersApplication::new(
            tracked_tickers_adapter.clone(),
            tracked_tickers_adapter.clone(),
        ),
        sector_performance: SectorPerformanceApplication::new(
            DuckDbMarketHistoryAdapter::new("data/market_data.duckdb"),
            ExchangeTradingCalendarAdapter,
        ),
        simulation: SimulationApplication,
        synchronization: SynchronizationApplication::new(
            YahooMarketHistoryAdapter,
            CboeOptionChainsAdapter,
            CboeVolatilityIndicesAdapter,
            TreasuryYieldCurvesAdapter,
            SynchronizationStores::new(
                DuckDbMarketHistoryAdapter::new("data/market_data.duckdb"),
                DuckDbOptionChainsAdapter::new("data/market_data.duckdb"),
                DuckDbVolatilityTermStructuresAdapter::new("data/market_data.duckdb"),
                DuckDbIndexHistoryAdapter::new("data/market_data.duckdb"),
                DuckDbYieldCurvesAdapter::new("data/market_data.duckdb"),
            ),
            tracked_tickers_adapter,
            OptionAnalysisCollaborators::new(
                DuckDbOptionChainsAdapter::new("data/market_data.duckdb"),
                DuckDbYieldCurvesAdapter::new("data/market_data.duckdb"),
                ExchangeTradingCalendarAdapter,
            ),
        ),
    }
}

/// Configures the live-price conversation, which has no storage dependency.
pub fn configure_market_stream() -> ConfiguredMarketStream {
    MarketStreamApplication::new(YahooLivePricesAdapter)
}

/// Wires the temporary offline migration from the legacy proof of concept.
///
/// SQLite is selected here only as a migration source; it is not part of the
/// production option-chain conversation.
pub fn configure_option_chain_migration(pool: SqlitePool) -> ConfiguredOptionChainMigration {
    OptionChainMigrationApplication::new(
        SqliteOptionDataAdapter::new(pool),
        DuckDbOptionChainsAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of prices, dividends, and splits.
pub fn configure_market_history_migration(pool: SqlitePool) -> ConfiguredMarketHistoryMigration {
    MarketHistoryMigrationApplication::new(
        SqliteMarketHistoryAdapter::new(pool),
        DuckDbMarketHistoryAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of volatility-index histories.
pub fn configure_index_history_migration(pool: SqlitePool) -> ConfiguredIndexHistoryMigration {
    IndexHistoryMigrationApplication::new(
        SqliteIndexHistoryAdapter::new(pool),
        DuckDbIndexHistoryAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of risk-free yield curves.
pub fn configure_yield_curve_migration(pool: SqlitePool) -> ConfiguredYieldCurveMigration {
    YieldCurveMigrationApplication::new(
        SqliteYieldCurvesAdapter::new(pool),
        DuckDbYieldCurvesAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of calculated volatility structures.
pub fn configure_volatility_term_structure_migration(
    pool: SqlitePool,
) -> ConfiguredVolatilityTermStructureMigration {
    VolatilityTermStructureMigrationApplication::new(
        SqliteOptionDataAdapter::new(pool),
        DuckDbVolatilityTermStructuresAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of tracked ticker configuration.
pub fn configure_tracked_ticker_migration(pool: SqlitePool) -> ConfiguredTrackedTickerMigration {
    TrackedTickerMigrationApplication::new(
        SqliteTrackedTickersAdapter::new(pool),
        DuckDbTrackedTickersAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of portfolio event ledgers.
pub fn configure_portfolio_migration(pool: SqlitePool) -> ConfiguredPortfolioMigration {
    PortfolioMigrationApplication::new(
        SqlitePortfolioAdapter::new(pool),
        DuckDbPortfolioAdapter::new("data/market_data.duckdb"),
    )
}

/// Wires the temporary offline migration of saved strategy definitions.
pub fn configure_strategy_migration(pool: SqlitePool) -> ConfiguredStrategyMigration {
    StrategyMigrationApplication::new(
        SqliteSavedStrategiesAdapter::new(pool),
        DuckDbSavedStrategiesAdapter::new("data/market_data.duckdb"),
    )
}

/// Connects the configured application to its production HTTP driving adapter.
pub fn configure_http() -> Router {
    let configured = configure();
    crate::driving_adapters::http::router(
        crate::driving_adapters::http::MarketViewingPorts::new(
            Arc::new(configured.market_data),
            Arc::new(configured.sector_performance),
        ),
        Arc::new(configured.options),
        Arc::new(configured.simulation),
        Arc::new(configured.portfolios),
        Arc::new(configured.synchronization),
        Arc::new(configured.saved_strategies),
        Arc::new(configured.tracked_tickers),
    )
}
