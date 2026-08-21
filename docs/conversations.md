# Conversation catalogue

A conversation is described from the application's point of view. The driving
actor begins it through a **driving port**. An application component implements
that port, coordinates domain behavior, and calls any required **driven ports**.
The configurator selects concrete adapters only at runtime composition.

## Offered conversations

| Conversation/use case | Typical driving actor | Driving port | Application coordinator | Required driven ports |
| --- | --- | --- | --- | --- |
| View market history, index history, or a live price | Market viewer | `ForViewingMarketData` | `MarketDataApplication` | `ForLoadingMarketHistory`, `ForLoadingIndexHistory`, `ForObtainingLivePrices` |
| Stream live prices | Live-price subscriber | `ForStreamingMarketPrices` | `MarketStreamApplication` | `ForStreamingLivePrices` |
| Inspect exchange scheduling | Scheduler | `ForSchedulingMarketOperations` | `MarketSchedulingApplication` | `ForConsultingTradingCalendar` |
| Analyze stored options | Options analyst | `ForAnalyzingOptions` | `OptionsApplication` | `ForLoadingOptionChains`, `ForLoadingVolatilityTermStructures`, `ForLoadingReferencePrices`, `ForLoadingYieldCurves`, `ForConsultingTradingCalendar` |
| Prepare an intraday option market | Intraday analyst | `ForPreparingIntradaySimulations`, `ForViewingIntradayOptions` | `IntradaySimulationApplication` | `ForObtainingOptionChains`, `ForObtainingLivePrices`, `ForConsultingTradingCalendar` |
| View a yield curve or interpolated rate | Interest-rate viewer | `ForViewingInterestRates` | `InterestRatesApplication` | `ForLoadingYieldCurves` |
| View market volatility | Volatility viewer | `ForViewingVolatility` | `MarketVolatilityApplication` | `ForLoadingIndexHistory`, `ForLoadingVolatilityTermStructures`, `ForLoadingMarketHistory` |
| Manage a portfolio | Portfolio owner | `ForManagingPortfolios` | `PortfolioApplication` | `ForLoadingPortfolios`, `ForStoringPortfolios` |
| Value portfolio positions | Portfolio viewer | `ForViewingPortfolioPositions` | `PortfolioValuationApplication` | `ForLoadingPortfolios`, `ForConsultingTradingCalendar`, `ForObtainingLivePrices`, `ForLoadingMarketHistory`, `ForObtainingOptionChains`, `ForLoadingOptionChains` |
| Manage saved strategies | Strategy-library owner | `ForManagingSavedStrategies` | `SavedStrategiesApplication` | `ForLoadingStrategies`, `ForStoringStrategies` |
| Manage tracked tickers | Market-data operator | `ForManagingTrackedTickers` | `TrackedTickersApplication` | `ForLoadingTrackedTickers`, `ForStoringTrackedTickers` |
| Simulate option strategies | Strategy analyst | `ForSimulatingStrategies` | `SimulationApplication` | None; calculation uses supplied domain values |
| Synchronize external market data | Data operator or scheduler | `ForSynchronizingMarketData` | `SynchronizationApplication` | Provider, persistence, ticker, option-data, and calendar ports described below |

## What coordination means

The application owns sequencing and business decisions. It may:

1. validate a request and load the required domain state;
2. choose which driven conversation is appropriate;
3. combine results from several actors;
4. invoke domain behavior and calculations;
5. persist the resulting state;
6. return a technology-neutral result through the driving port.

Adapters perform translation and I/O for their technology. They do not select
another adapter, implement a complete cross-actor workflow, or expose provider
DTOs to the application.

## Detailed conversations

- [Portfolio valuation](conversations/portfolio-valuation.md) demonstrates a
  conditional choice between live and stored observations.
- [Market-data synchronization](conversations/synchronization.md) demonstrates
  acquisition, calculation, and persistence across several actors.
- [Option analysis](conversations/options.md) demonstrates multiple operations
  forming one coherent driving conversation.
- [Intraday market data](conversations/intraday-market.md) contrasts orchestration
  with a simple delegated streaming conversation.
