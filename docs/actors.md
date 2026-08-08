# Actor catalogue

An **actor** is a participant outside the hexagon. An **adapter** is technical
code that translates between an actor and a port. They are deliberately not
synonyms: replacing Axum with another web framework changes an adapter, not the
human or system driving the conversation; replacing SQLite with PostgreSQL
changes a driven adapter, not the application's need for persistence.

```text
driving actor -> driving adapter -> driving port
                    HEXAGON
driven actor  <- driven adapter  <- driven port
```

The application knows port contracts and domain values. It does not know any
adapter or technology listed below.

## Driving actors

| Actor | Intent | Possible/current adapter | Driving ports used |
| --- | --- | --- | --- |
| Market viewer | Inspect stored history, indices, and a current price | Axum HTTP; future Leptos SSR | `ForViewingMarketData` |
| Options analyst | Inspect chains, term structure, surface, skew, and Greeks | Axum HTTP; future Leptos SSR | `ForAnalyzingOptions` |
| Intraday analyst | Prepare a live option-market view | Web server or future UI | `ForViewingIntradayOptions`, `ForPreparingIntradaySimulations` |
| Portfolio owner | Create a portfolio and record or inspect its activity | Axum HTTP; future Leptos SSR | `ForManagingPortfolios` |
| Portfolio viewer | Inspect positions valued at current or end-of-day prices | Web server or future Leptos SSR | `ForViewingPortfolioPositions` |
| Strategy analyst | Build and run option-strategy scenarios | Axum HTTP; future Leptos SSR | `ForSimulatingStrategies` |
| Strategy-library owner | List, save, and delete reusable strategies | Axum HTTP; future Leptos SSR | `ForManagingSavedStrategies` |
| Market-data operator | Configure the instruments maintained by the backend | Axum HTTP or CLI | `ForManagingTrackedTickers` |
| Data operator | Request one synchronization operation | Axum HTTP or CLI | `ForSynchronizingMarketData` |
| Scheduler | Decide when and initiate market operations | Process scheduler | `ForSchedulingMarketOperations`, `ForSynchronizingMarketData` |
| Interest-rate viewer | Inspect curves and interpolated rates | Web server or future UI | `ForViewingInterestRates` |
| Volatility viewer | Inspect implied and historical volatility | Web server or future UI | `ForViewingVolatility` |
| Live-price subscriber | Receive a changing stream of prices | WebSocket-facing server | `ForStreamingMarketPrices` |
| Automated test | Exercise an offered conversation deterministically | In-process test adapter | Any driving port |

The same actor may use several driving ports. Conversely, HTTP, CLI, a scheduler,
and an automated test can all drive the same port without changing the use case.

## Driven actors

| Actor | Role in conversations | Driven ports | Production adapter(s) |
| --- | --- | --- | --- |
| SQLite database | Temporary migration source and inactive proof of concept | Contract-tested domain ports and migration-only archive ports | Domain-focused `Sqlite*Adapter` implementations |
| DuckDB database | Production persistence for all application-owned data | Domain-focused loading, storing, and migration-verification ports | Domain-focused `DuckDb*Adapter` implementations |
| Yahoo Finance | Supply historical market data, current prices, and price streams | `ForObtainingMarketHistory`, `ForObtainingLivePrices`, `ForStreamingLivePrices` | `YahooMarketHistoryAdapter`, `YahooLivePricesAdapter` |
| Cboe | Supply option chains and volatility-index histories | `ForObtainingOptionChains`, `ForObtainingVolatilityIndices` | `CboeOptionChainsAdapter`, `CboeVolatilityIndicesAdapter` |
| U.S. Treasury | Supply published risk-free yield curves | `ForObtainingYieldCurves` | `TreasuryYieldCurvesAdapter` |
| Exchange calendar | Answer exchange-session questions | `ForConsultingTradingCalendar` | `ExchangeTradingCalendarAdapter` |
| Test double | Replace any driven actor in application tests | The port required by the test | Mock, stub, or in-memory fake |

One port may have multiple adapters. For example, a future PostgreSQL or
in-memory adapter can implement `ForLoadingPortfolios`. One adapter may also
implement several coherent ports: `DuckDbPortfolioAdapter` implements both
loading and storing for the same aggregate and technology.

What an adapter must not do is coordinate unrelated actors. Choosing between
Yahoo, Cboe, DuckDB, and the exchange calendar is application work.
