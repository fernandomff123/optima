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
        exchange_calendar::ExchangeTradingCalendarAdapter,
        instrument_prices::MarketInstrumentPricesAdapter,
        sqlite::{
            index_history_port::SqliteIndexHistoryAdapter, market_data::SqliteMarketDataAdapter,
            market_history::SqliteMarketHistoryAdapter, option_data::SqliteOptionDataAdapter,
            portfolio::SqlitePortfolioAdapter, saved_strategies::SqliteSavedStrategiesAdapter,
            tracked_tickers::SqliteTrackedTickersAdapter,
            yield_curves_port::SqliteYieldCurvesAdapter,
        },
        treasury::TreasuryYieldCurvesAdapter,
        yahoo::{YahooLivePricesAdapter, YahooMarketHistoryAdapter},
    },
    hexagon::application::{
        interest_rates::InterestRatesApplication,
        intraday_simulation::IntradaySimulationApplication, market_data::MarketDataApplication,
        market_scheduling::MarketSchedulingApplication, market_stream::MarketStreamApplication,
        market_volatility::MarketVolatilityApplication, options::OptionsApplication,
        portfolio::PortfolioApplication, portfolio_valuation::PortfolioValuationApplication,
        saved_strategies::SavedStrategiesApplication, simulation::SimulationApplication,
        synchronization::OptionAnalysisCollaborators, synchronization::SynchronizationApplication,
        tracked_tickers::TrackedTickersApplication,
    },
};

pub type ConfiguredMarketData = MarketDataApplication<
    SqliteMarketHistoryAdapter,
    SqliteIndexHistoryAdapter,
    YahooLivePricesAdapter,
>;
pub type ConfiguredMarketStream = MarketStreamApplication<YahooLivePricesAdapter>;
pub type ConfiguredMarketScheduling = MarketSchedulingApplication<ExchangeTradingCalendarAdapter>;
pub type ConfiguredOptions =
    OptionsApplication<SqliteOptionDataAdapter, ExchangeTradingCalendarAdapter>;
pub type ConfiguredInterestRates = InterestRatesApplication<SqliteYieldCurvesAdapter>;
pub type ConfiguredMarketVolatility = MarketVolatilityApplication<
    SqliteIndexHistoryAdapter,
    SqliteOptionDataAdapter,
    SqliteMarketHistoryAdapter,
>;
pub type ConfiguredIntradaySimulation = IntradaySimulationApplication<
    CboeOptionChainsAdapter,
    YahooLivePricesAdapter,
    ExchangeTradingCalendarAdapter,
>;
pub type ConfiguredPortfolios =
    PortfolioApplication<SqlitePortfolioAdapter, SqlitePortfolioAdapter>;
pub type ConfiguredPortfolioValuation =
    PortfolioValuationApplication<SqlitePortfolioAdapter, MarketInstrumentPricesAdapter>;
pub type ConfiguredSavedStrategies =
    SavedStrategiesApplication<SqliteSavedStrategiesAdapter, SqliteSavedStrategiesAdapter>;
pub type ConfiguredTrackedTickers =
    TrackedTickersApplication<SqliteTrackedTickersAdapter, SqliteTrackedTickersAdapter>;
pub type ConfiguredSynchronization = SynchronizationApplication<
    YahooMarketHistoryAdapter,
    CboeOptionChainsAdapter,
    CboeVolatilityIndicesAdapter,
    TreasuryYieldCurvesAdapter,
    SqliteMarketDataAdapter,
    SqliteTrackedTickersAdapter,
    SqliteOptionDataAdapter,
    ExchangeTradingCalendarAdapter,
>;

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

/// Selects concrete production adapters and injects them through constructors.
pub fn configure(pool: SqlitePool) -> ConfiguredApplication {
    let portfolio_adapter = SqlitePortfolioAdapter::new(pool.clone());
    let strategies_adapter = SqliteSavedStrategiesAdapter::new(pool.clone());
    let tracked_tickers_adapter = SqliteTrackedTickersAdapter::new(pool.clone());
    ConfiguredApplication {
        market_data: MarketDataApplication::new(
            SqliteMarketHistoryAdapter::new(pool.clone()),
            SqliteIndexHistoryAdapter::new(pool.clone()),
            YahooLivePricesAdapter,
        ),
        market_stream: configure_market_stream(),
        market_scheduling: MarketSchedulingApplication::new(ExchangeTradingCalendarAdapter),
        interest_rates: InterestRatesApplication::new(SqliteYieldCurvesAdapter::new(pool.clone())),
        market_volatility: MarketVolatilityApplication::new(
            SqliteIndexHistoryAdapter::new(pool.clone()),
            SqliteOptionDataAdapter::new(pool.clone()),
            SqliteMarketHistoryAdapter::new(pool.clone()),
        ),
        intraday_simulation: IntradaySimulationApplication::new(
            CboeOptionChainsAdapter,
            YahooLivePricesAdapter,
            ExchangeTradingCalendarAdapter,
        ),
        options: OptionsApplication::new(
            SqliteOptionDataAdapter::new(pool.clone()),
            ExchangeTradingCalendarAdapter,
        ),
        portfolios: PortfolioApplication::new(portfolio_adapter.clone(), portfolio_adapter),
        portfolio_valuation: PortfolioValuationApplication::new(
            SqlitePortfolioAdapter::new(pool.clone()),
            MarketInstrumentPricesAdapter::new(pool.clone()),
        ),
        saved_strategies: SavedStrategiesApplication::new(
            strategies_adapter.clone(),
            strategies_adapter,
        ),
        tracked_tickers: TrackedTickersApplication::new(
            tracked_tickers_adapter.clone(),
            tracked_tickers_adapter.clone(),
        ),
        simulation: SimulationApplication,
        synchronization: SynchronizationApplication::new(
            YahooMarketHistoryAdapter,
            CboeOptionChainsAdapter,
            CboeVolatilityIndicesAdapter,
            TreasuryYieldCurvesAdapter,
            SqliteMarketDataAdapter::new(pool.clone()),
            tracked_tickers_adapter,
            OptionAnalysisCollaborators::new(
                SqliteOptionDataAdapter::new(pool.clone()),
                ExchangeTradingCalendarAdapter,
            ),
        ),
    }
}

/// Configures the live-price conversation, which has no storage dependency.
pub fn configure_market_stream() -> ConfiguredMarketStream {
    MarketStreamApplication::new(YahooLivePricesAdapter)
}

/// Connects the configured application to its production HTTP driving adapter.
pub fn configure_http(pool: SqlitePool) -> Router {
    let configured = configure(pool);
    crate::driving_adapters::http::router(
        Arc::new(configured.market_data),
        Arc::new(configured.options),
        Arc::new(configured.simulation),
        Arc::new(configured.portfolios),
        Arc::new(configured.synchronization),
        Arc::new(configured.saved_strategies),
        Arc::new(configured.tracked_tickers),
    )
}
