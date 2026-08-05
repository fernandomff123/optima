# Ports & Adapters architecture

This backend is one application and therefore one hexagon. Its internal
business logic is split into cohesive modules, but those modules are not nested
hexagons. This follows *Hexagonal Architecture Explained*, especially chapters
2.1–2.4, 4.3–4.9, 5.3–5.4, and 5.8.

## Boundary

The application owns all port definitions. Nothing crosses the application
boundary except through a port. Code inside the hexagon cannot depend on Axum,
SQLx, Reqwest, SQLite, wire formats, or provider names such as Yahoo and CBOE.

```text
src/
├── hexagon/
│   ├── application/
│   │   ├── interest_rates/
│   │   ├── intraday_simulation/
│   │   ├── market_data/
│   │   ├── market_scheduling/
│   │   ├── market_stream/
│   │   ├── market_volatility/
│   │   ├── options/
│   │   ├── portfolio/
│   │   ├── portfolio_valuation/
│   │   ├── saved_strategies/
│   │   ├── simulation/
│   │   ├── synchronization/
│   │   └── tracked_tickers/
│   ├── domain/
│   ├── driving_ports/
│   │   └── for_doing_something/
│   └── driven_ports/
│       └── for_doing_something/
├── driving_adapters/
├── driven_adapters/
│   ├── cboe/                 # option-chain and volatility-index adapters
│   ├── exchange_calendar/    # official trading-session adapter
│   ├── sqlite/               # persistence adapters
│   ├── treasury/             # risk-free yield-curve adapter
│   └── yahoo/                # historical and live-price adapters
└── configurator/
```

There is deliberately no `services/`, `storage/`, or provider-specific port
folder. Persistence is a driven actor reached through a driven port, just like
an HTTP data provider. A business module may participate in several coherent
conversations and therefore use or provide more than one port.

Ports are grouped by the intention of a conversation. Their names use the
`ForDoingSomething` convention; Rust modules use `for_doing_something`. A port
name never mentions an adapter technology or a provider.

Financial calculations are domain code. In particular, interest-rate
interpolation and option-volatility term-structure calculation live under
`hexagon/domain`. The volatility calculation receives a time to expiration; it
does not consult an exchange calendar itself. Resolving session open/close
times is an external conversation represented by
`ForConsultingTradingCalendar` and implemented by a driven adapter.

## Driving actors and ports

| Actor/intention | Driving port |
| --- | --- |
| Market viewer (historical and current prices) | `ForViewingMarketData` |
| Options analyst | `ForAnalyzingOptions` |
| Intraday options viewer | `ForViewingIntradayOptions` |
| Intraday simulation client | `ForPreparingIntradaySimulations` |
| Portfolio owner | `ForManagingPortfolios` |
| Portfolio positions viewer | `ForViewingPortfolioPositions` |
| Strategy library owner | `ForManagingSavedStrategies` |
| Market-data operator | `ForManagingTrackedTickers` |
| Strategy analyst | `ForSimulatingStrategies` |
| Interest-rate viewer | `ForViewingInterestRates` |
| Volatility viewer | `ForViewingVolatility` |
| Live-price subscriber | `ForStreamingMarketPrices` |
| Market-operation scheduler | `ForSchedulingMarketOperations` |
| Data operator or scheduler | `ForSynchronizingMarketData` |

HTTP, WebSocket, command-line programs, schedulers, and tests may drive these
ports. They are actors or adapters, not part of the application.

The Axum adapter in `driving_adapters/http` depends only on driving ports. Its
contract tests replace the application with mocks and verify HTTP translation.

## Driven actors and ports

| Application need | Driven port | Production adapter |
| --- | --- | --- |
| Store market history | `ForStoringMarketHistory` | SQLite |
| Store option data and derived analytics | `ForStoringOptionData` | SQLite |
| Store volatility-index history | `ForStoringIndexHistory` | SQLite |
| Store risk-free yield curves | `ForStoringYieldCurves` | SQLite |
| Load stored market history | `ForLoadingMarketHistory` | SQLite |
| Load stored index history | `ForLoadingIndexHistory` | SQLite |
| Load stored option data | `ForLoadingOptionData` | SQLite |
| Load stored yield curves | `ForLoadingYieldCurves` | SQLite |
| Load portfolios | `ForLoadingPortfolios` | SQLite |
| Store portfolios | `ForStoringPortfolios` | SQLite |
| Store strategies | `ForStoringStrategies` | SQLite |
| Load strategies | `ForLoadingStrategies` | SQLite |
| Load tracked tickers | `ForLoadingTrackedTickers` | SQLite |
| Store tracked tickers | `ForStoringTrackedTickers` | SQLite |
| Obtain historical prices and corporate actions | `ForObtainingMarketHistory` | Yahoo |
| Obtain live prices | `ForObtainingLivePrices` | Yahoo |
| Stream live prices | `ForStreamingLivePrices` | Yahoo |
| Obtain prices for held instruments | `ForObtainingInstrumentPrices` | market-price adapter |
| Obtain option chains | `ForObtainingOptionChains` | CBOE |
| Obtain volatility-index history | `ForObtainingVolatilityIndices` | CBOE |
| Obtain risk-free yield curves | `ForObtainingYieldCurves` | U.S. Treasury |
| Consult exchange sessions | `ForConsultingTradingCalendar` | exchange calendar library |

The adapter column is configuration, not part of any port contract. Tests
replace each production adapter with an in-memory test double implementing the
same driven port.

Provider aliases and wire formats are translated at the adapter boundary. For
example, the Yahoo adapter maps the domain ticker `SPX` to `^GSPC` and restores
`SPX` before returning the domain object. Neither alias is present in a port.

## Dependency direction

1. Driving actors or adapters depend on driving ports.
2. The application implements driving ports.
3. The application depends on driven ports supplied through constructors.
4. Driven adapters implement driven ports.
5. The configurator constructs driven adapters, injects them into the
   application, and gives the application to driving adapters.

The configurator is the only code allowed to know every concrete participant.
The production `configure` function builds one `ConfiguredApplication`; its
fields expose the driving ports to HTTP, CLI, scheduler, and test adapters.
`configure_http` then hands those provided interfaces to the Axum adapter.

The production web-server bootstrap mounts this configured router directly.
The existing `/api` wire contract is preserved by compatibility mappers under
`driving_adapters/http`. Those mappers translate between `api_models` and
domain/application types; they perform no persistence or provider I/O. Every
route obtains data and executes behavior through a driving port, so transport
compatibility does not change the dependency direction.

## Simulation conversation

`ForSimulatingStrategies` is intentionally not a one-function-per-port design.
It offers the related operations needed by a strategy-analysis actor: building
a scenario grid, simulating an already prepared strategy, and executing the
complete `SimulateScenario` use case. The complete command contains domain
snapshots, a yield curve, requested dates, shocks, and leg selections; it does
not contain HTTP DTOs or provider names. The application returns
`SimulationScenario`, which the HTTP adapter maps to the existing wire model.

## Testing strategy

Development follows the book's sequence:

1. **Test-to-test:** tests drive real application use cases while all driven
   actors are mocks, stubs, fakes, or other in-memory test doubles.
2. **Real-to-test:** production driving adapters are tested against a fake
   application or test doubles.
3. **Test-to-real:** adapter contract tests exercise SQLite and recorded wire
   fixtures.
4. **Real-to-real:** opt-in integration tests exercise production providers.

Architecture tests reject technology and adapter dependencies from the
hexagon. HTTP contract tests preserve the existing API behavior during the
migration.

## Generated API documentation

Generate and validate the Rust API documentation from the repository root:

```bash
env RUSTDOCFLAGS=-Dwarnings \
  cargo +stable doc --workspace --all-features --no-deps --offline
```

The autonomous backend crate is published locally at
`target/doc/hexagonal_backend/index.html`. Both `target/` and the SQLite file
under `data/` belong to this repository at runtime but are intentionally not
committed.
