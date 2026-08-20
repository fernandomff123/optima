//! Composition root that knows and connects all concrete participants.
//!
//! No application module constructs an adapter. This outer component is the
//! only place where provider and database choices are combined.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::Router;
use sqlx::SqlitePool;

use crate::hexagon::driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers;
use crate::hexagon::driving_ports::for_refreshing_market_data::ForRefreshingMarketData;
use crate::{
    driven_adapters::{
        cboe::{
            CboeOptionChainsAdapter, CboeVolatilityIndicesAdapter,
            product_specifications::CboeOptionContractSpecificationsAdapter,
        },
        data_refresh_tasks::TokioDataRefreshTaskRunner,
        duckdb::{
            data_refresh_runs::DuckDbDataRefreshRunsAdapter,
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
        yahoo::{
            YahooLivePricesAdapter, YahooMarketHistoryAdapter, YahooUnderlyingResolverAdapter,
        },
    },
    hexagon::application::{
        data_refresh::DataRefreshApplication,
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
            OptionAnalysisCollaborators, OptionSnapshotEnrichment, SynchronizationApplication,
            SynchronizationSources, SynchronizationStores,
        },
        tracked_ticker_migration::TrackedTickerMigrationApplication,
        tracked_tickers::TrackedTickersApplication,
        volatility_term_structure_migration::VolatilityTermStructureMigrationApplication,
        yield_curve_migration::YieldCurveMigrationApplication,
    },
};

pub const DUCKDB_PATH_ENV: &str = "HEXAGONAL_BACKEND_DUCKDB_PATH";
pub const DEFAULT_DUCKDB_PATH: &str = "data/market_data.duckdb";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionConfig {
    duckdb_path: PathBuf,
}

impl Default for CompositionConfig {
    fn default() -> Self {
        Self {
            duckdb_path: PathBuf::from(DEFAULT_DUCKDB_PATH),
        }
    }
}

impl CompositionConfig {
    pub fn with_duckdb_path(path: impl Into<PathBuf>) -> Self {
        Self {
            duckdb_path: path.into(),
        }
    }
    pub fn from_environment() -> Self {
        Self::from_path_override(std::env::var_os(DUCKDB_PATH_ENV))
    }
    pub fn from_path_override(value: Option<std::ffi::OsString>) -> Self {
        value
            .filter(|path| !path.is_empty())
            .map(Self::with_duckdb_path)
            .unwrap_or_default()
    }
    pub fn duckdb_path(&self) -> &Path {
        &self.duckdb_path
    }
}

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
pub type ConfiguredTrackedTickers = TrackedTickersApplication<
    DuckDbTrackedTickersAdapter,
    DuckDbTrackedTickersAdapter,
    YahooUnderlyingResolverAdapter,
>;
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
    CboeOptionContractSpecificationsAdapter,
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
    pub composition_config: CompositionConfig,
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
    pub synchronization: Arc<ConfiguredSynchronization>,
    pub data_refresh: Arc<dyn ForRefreshingMarketData>,
    #[cfg(test)]
    data_refresh_application: Arc<DataRefreshApplication>,
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
    let tracked_tickers_adapter = SqliteTrackedTickersAdapter::new(pool.clone());
    TrackedTickersApplication::new(
        tracked_tickers_adapter.clone(),
        tracked_tickers_adapter,
        YahooUnderlyingResolverAdapter::default(),
    )
    .bootstrap_system_tickers()
    .await
    .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
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
    initialize_analytical_storage_with_config(&CompositionConfig::from_environment()).await
}

pub async fn initialize_analytical_storage_with_config(
    config: &CompositionConfig,
) -> crate::hexagon::PortResult<()> {
    let path = config.duckdb_path();
    DuckDbOptionChainsAdapter::new(path).initialize().await?;
    DuckDbMarketHistoryAdapter::new(path).initialize().await?;
    DuckDbIndexHistoryAdapter::new(path).initialize().await?;
    DuckDbYieldCurvesAdapter::new(path).initialize().await?;
    DuckDbVolatilityTermStructuresAdapter::new(path)
        .initialize()
        .await?;
    let tracked_tickers = DuckDbTrackedTickersAdapter::new(path);
    tracked_tickers.initialize().await?;
    TrackedTickersApplication::new(
        tracked_tickers.clone(),
        tracked_tickers,
        YahooUnderlyingResolverAdapter::default(),
    )
    .bootstrap_system_tickers()
    .await?;
    DuckDbPortfolioAdapter::new(path).initialize().await?;
    DuckDbSavedStrategiesAdapter::new(path).initialize().await?;
    DuckDbDataRefreshRunsAdapter::new(path).initialize().await
}

/// Selects concrete production adapters and injects them through constructors.
pub fn configure() -> ConfiguredApplication {
    configure_with_config(&CompositionConfig::from_environment())
}

pub fn configure_with_config(config: &CompositionConfig) -> ConfiguredApplication {
    let path = config.duckdb_path();
    let portfolio_adapter = DuckDbPortfolioAdapter::new(path);
    let strategies_adapter = DuckDbSavedStrategiesAdapter::new(path);
    let tracked_tickers_adapter = DuckDbTrackedTickersAdapter::new(path);
    let synchronization = Arc::new(configure_synchronization(
        path,
        tracked_tickers_adapter.clone(),
    ));
    let refresh_runs = Arc::new(DuckDbDataRefreshRunsAdapter::new(path));
    let data_refresh_application = Arc::new(DataRefreshApplication::new(
        synchronization.clone(),
        refresh_runs,
        Arc::new(DuckDbMarketHistoryAdapter::new(path)),
        Arc::new(tracked_tickers_adapter.clone()),
        Arc::new(ExchangeTradingCalendarAdapter),
        Arc::new(TokioDataRefreshTaskRunner),
    ));
    let data_refresh: Arc<dyn ForRefreshingMarketData> = data_refresh_application.clone();
    ConfiguredApplication {
        composition_config: config.clone(),
        market_data: MarketDataApplication::new(
            DuckDbMarketHistoryAdapter::new(path),
            DuckDbIndexHistoryAdapter::new(path),
            YahooLivePricesAdapter,
        ),
        market_stream: configure_market_stream(),
        market_scheduling: MarketSchedulingApplication::new(ExchangeTradingCalendarAdapter),
        interest_rates: InterestRatesApplication::new(DuckDbYieldCurvesAdapter::new(path)),
        market_volatility: MarketVolatilityApplication::new(
            DuckDbIndexHistoryAdapter::new(path),
            DuckDbVolatilityTermStructuresAdapter::new(path),
            DuckDbMarketHistoryAdapter::new(path),
        ),
        intraday_simulation: IntradaySimulationApplication::new(
            CboeOptionChainsAdapter,
            YahooLivePricesAdapter,
            ExchangeTradingCalendarAdapter,
        ),
        options: OptionsApplication::new(
            DuckDbOptionChainsAdapter::new(path),
            DuckDbVolatilityTermStructuresAdapter::new(path),
            DuckDbYieldCurvesAdapter::new(path),
            ExchangeTradingCalendarAdapter,
        ),
        portfolios: PortfolioApplication::new(portfolio_adapter.clone(), portfolio_adapter),
        portfolio_valuation: PortfolioValuationApplication::new(
            DuckDbPortfolioAdapter::new(path),
            PricingCollaborators::new(
                ExchangeTradingCalendarAdapter,
                YahooLivePricesAdapter,
                DuckDbMarketHistoryAdapter::new(path),
                CboeOptionChainsAdapter,
                DuckDbOptionChainsAdapter::new(path),
            ),
        ),
        saved_strategies: SavedStrategiesApplication::new(
            strategies_adapter.clone(),
            strategies_adapter,
        ),
        tracked_tickers: TrackedTickersApplication::new(
            tracked_tickers_adapter.clone(),
            tracked_tickers_adapter.clone(),
            YahooUnderlyingResolverAdapter::default(),
        ),
        sector_performance: SectorPerformanceApplication::new(
            DuckDbMarketHistoryAdapter::new(path),
            ExchangeTradingCalendarAdapter,
        ),
        simulation: SimulationApplication,
        synchronization,
        data_refresh,
        #[cfg(test)]
        data_refresh_application,
    }
}

fn configure_synchronization(
    path: &Path,
    tracked_tickers_adapter: DuckDbTrackedTickersAdapter,
) -> ConfiguredSynchronization {
    SynchronizationApplication::new(
        SynchronizationSources::new(
            YahooMarketHistoryAdapter,
            CboeOptionChainsAdapter,
            CboeVolatilityIndicesAdapter,
            TreasuryYieldCurvesAdapter,
        ),
        SynchronizationStores::new(
            DuckDbMarketHistoryAdapter::new(path),
            DuckDbOptionChainsAdapter::new(path),
            DuckDbVolatilityTermStructuresAdapter::new(path),
            DuckDbIndexHistoryAdapter::new(path),
            DuckDbYieldCurvesAdapter::new(path),
        ),
        tracked_tickers_adapter,
        OptionAnalysisCollaborators::new(
            DuckDbOptionChainsAdapter::new(path),
            DuckDbYieldCurvesAdapter::new(path),
            ExchangeTradingCalendarAdapter,
        ),
        OptionSnapshotEnrichment::new(CboeOptionContractSpecificationsAdapter),
    )
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
        DuckDbOptionChainsAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Wires the temporary offline migration of prices, dividends, and splits.
pub fn configure_market_history_migration(pool: SqlitePool) -> ConfiguredMarketHistoryMigration {
    MarketHistoryMigrationApplication::new(
        SqliteMarketHistoryAdapter::new(pool),
        DuckDbMarketHistoryAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Wires the temporary offline migration of volatility-index histories.
pub fn configure_index_history_migration(pool: SqlitePool) -> ConfiguredIndexHistoryMigration {
    IndexHistoryMigrationApplication::new(
        SqliteIndexHistoryAdapter::new(pool),
        DuckDbIndexHistoryAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Wires the temporary offline migration of risk-free yield curves.
pub fn configure_yield_curve_migration(pool: SqlitePool) -> ConfiguredYieldCurveMigration {
    YieldCurveMigrationApplication::new(
        SqliteYieldCurvesAdapter::new(pool),
        DuckDbYieldCurvesAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Wires the temporary offline migration of calculated volatility structures.
pub fn configure_volatility_term_structure_migration(
    pool: SqlitePool,
) -> ConfiguredVolatilityTermStructureMigration {
    VolatilityTermStructureMigrationApplication::new(
        SqliteOptionDataAdapter::new(pool),
        DuckDbVolatilityTermStructuresAdapter::new(
            CompositionConfig::from_environment().duckdb_path(),
        ),
    )
}

/// Wires the temporary offline migration of tracked ticker configuration.
pub fn configure_tracked_ticker_migration(pool: SqlitePool) -> ConfiguredTrackedTickerMigration {
    TrackedTickerMigrationApplication::new(
        SqliteTrackedTickersAdapter::new(pool),
        DuckDbTrackedTickersAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Wires the temporary offline migration of portfolio event ledgers.
pub fn configure_portfolio_migration(pool: SqlitePool) -> ConfiguredPortfolioMigration {
    PortfolioMigrationApplication::new(
        SqlitePortfolioAdapter::new(pool),
        DuckDbPortfolioAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Wires the temporary offline migration of saved strategy definitions.
pub fn configure_strategy_migration(pool: SqlitePool) -> ConfiguredStrategyMigration {
    StrategyMigrationApplication::new(
        SqliteSavedStrategiesAdapter::new(pool),
        DuckDbSavedStrategiesAdapter::new(CompositionConfig::from_environment().duckdb_path()),
    )
}

/// Connects the configured application to its production HTTP driving adapter.
pub fn configure_http() -> Router {
    configure_http_application(configure())
}

pub fn configure_http_application(configured: ConfiguredApplication) -> Router {
    let (_market_session_updates, market_session) = tokio::sync::watch::channel(false);
    configure_server_http_application(configured, market_session)
}

/// Connects the single configured application to every current HTTP route.
///
/// The receiver is runtime state, not a dependency selected by the adapter.
pub fn configure_server_http_application(
    configured: ConfiguredApplication,
    market_session: tokio::sync::watch::Receiver<bool>,
) -> Router {
    let ConfiguredApplication {
        market_data,
        market_stream,
        market_scheduling: _,
        interest_rates,
        market_volatility,
        intraday_simulation,
        options,
        portfolios,
        portfolio_valuation,
        saved_strategies,
        tracked_tickers,
        sector_performance,
        simulation,
        synchronization,
        data_refresh,
        ..
    } = configured;

    let market_data = Arc::new(market_data);
    let options = Arc::new(options);
    let simulation = Arc::new(simulation);
    let portfolios = Arc::new(portfolios);
    let saved_strategies = Arc::new(saved_strategies);
    let tracked_tickers = Arc::new(tracked_tickers);
    let modern = crate::driving_adapters::http::router_with_data_refresh(
        crate::driving_adapters::http::MarketViewingPorts::new(
            market_data.clone(),
            Arc::new(sector_performance),
        ),
        options.clone(),
        simulation.clone(),
        portfolios.clone(),
        crate::driving_adapters::http::SynchronizationPorts::new(synchronization, data_refresh),
        saved_strategies.clone(),
        crate::driving_adapters::http::TrackedTickerPorts::new(
            tracked_tickers.clone(),
            tracked_tickers.clone(),
        ),
    );

    let intraday_simulation = Arc::new(intraday_simulation);
    let legacy = crate::driving_adapters::http::legacy_server::router(
        crate::driving_adapters::http::legacy_server::LegacyHttpPorts {
            market_data,
            market_stream: Arc::new(market_stream),
            market_volatility: Arc::new(market_volatility),
            interest_rates: Arc::new(interest_rates),
            portfolios,
            portfolio_valuation: Arc::new(portfolio_valuation),
            options,
            simulation,
            intraday_simulation: intraday_simulation.clone(),
            intraday_options: intraday_simulation,
            saved_strategies,
            tracked_tickers,
        },
        market_session,
    );

    modern.merge(legacy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::driving_ports::for_synchronizing_market_data::ForSynchronizingMarketData;

    #[test]
    fn data_refresh_and_http_share_the_configured_synchronization_arc() {
        let config = CompositionConfig::with_duckdb_path(
            std::env::temp_dir().join("shared-synchronization-composition.duckdb"),
        );
        let configured = configure_with_config(&config);
        let http_synchronization: Arc<dyn ForSynchronizingMarketData> =
            configured.synchronization.clone();

        assert!(Arc::ptr_eq(
            configured.data_refresh_application.synchronization_port(),
            &http_synchronization,
        ));
    }
}
