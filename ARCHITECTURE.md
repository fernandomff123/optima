# Ports & Adapters architecture

This backend is one application and therefore one hexagon. Its internal
business logic is split into cohesive modules, but those modules are not nested
hexagons. This follows *Hexagonal Architecture Explained*, especially chapters
2.1–2.4, 4.3–4.9, 5.3–5.4, and 5.8.

The architecture can also be explored by intent rather than by source tree:

- [Actor catalogue](docs/actors.md) identifies driving and driven actors and
  separates actors from their adapters.
- [Conversation catalogue](docs/conversations.md) maps every offered use case
  to its driving port, application coordinator, required ports, and actors.
- Detailed flows explain [portfolio valuation](docs/conversations/portfolio-valuation.md),
  [market-data synchronization](docs/conversations/synchronization.md),
  [option analysis](docs/conversations/options.md), and
  [intraday market data](docs/conversations/intraday-market.md).

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
│   ├── duckdb/               # columnar analytical persistence adapters
│   ├── exchange_calendar/    # official trading-session adapter
│   ├── sqlite/               # inactive proof-of-concept and migration sources
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
| S&P 500 sector-performance viewer | `ForViewingSectorPerformance` |
| Options analyst | `ForAnalyzingOptions` |
| Intraday options viewer | `ForViewingIntradayOptions` |
| Intraday simulation client | `ForPreparingIntradaySimulations` |
| Portfolio owner | `ForManagingPortfolios` |
| Portfolio positions viewer | `ForViewingPortfolioPositions` |
| Strategy library owner | `ForManagingSavedStrategies` |
| Market-data operator | `ForManagingTrackedTickers` |
| Underlying resolver client | `ForResolvingUnderlyings` |
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
| Store market history | `ForStoringMarketHistory` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Store option chains | `ForStoringOptionChains` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Store volatility term structures | `ForStoringVolatilityTermStructures` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Store volatility-index history | `ForStoringIndexHistory` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Store risk-free yield curves | `ForStoringYieldCurves` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Load stored market history | `ForLoadingMarketHistory` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Export market histories during migration | `ForLoadingMarketHistoryArchive` | SQLite (temporary migration source only) |
| Count market-history observations | `ForCountingMarketHistory` | DuckDB |
| Load stored index history | `ForLoadingIndexHistory` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Export index histories during migration | `ForLoadingIndexHistoryArchive` | SQLite (temporary migration source only) |
| Count index observations | `ForCountingIndexHistory` | DuckDB |
| Load stored option chains | `ForLoadingOptionChains` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Export archived option chains during migration | `ForLoadingOptionChainArchive` | SQLite (temporary migration source only) |
| Count option-chain observations | `ForCountingOptionChains` | DuckDB |
| Load volatility term structures | `ForLoadingVolatilityTermStructures` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Load underlying reference prices | `ForLoadingReferencePrices` | DuckDB |
| Export volatility structures during migration | `ForLoadingVolatilityTermStructureArchive` | SQLite (temporary migration source only) |
| Count volatility structure points | `ForCountingVolatilityTermStructures` | DuckDB |
| Load stored yield curves | `ForLoadingYieldCurves` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Export yield curves during migration | `ForLoadingYieldCurveArchive` | SQLite (temporary migration source only) |
| Count yield curves | `ForCountingYieldCurves` | DuckDB |
| Load portfolios | `ForLoadingPortfolios` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Store portfolios | `ForStoringPortfolios` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Export portfolio ledgers during migration | `ForLoadingPortfolioArchive` | SQLite (temporary migration source only) |
| Count portfolio ledgers | `ForCountingPortfolios` | DuckDB |
| Store strategies | `ForStoringStrategies` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Load strategies | `ForLoadingStrategies` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Import complete strategy records during migration | `ForImportingStrategyArchive` | DuckDB |
| Count saved strategies and legs | `ForCountingStrategies` | DuckDB |
| Load tracked tickers | `ForLoadingTrackedTickers` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Store tracked tickers | `ForStoringTrackedTickers` | DuckDB (SQLite proof-of-concept also passes the contract) |
| Export tracked ticker configuration during migration | `ForLoadingTrackedTickerArchive` | SQLite (temporary migration source only) |
| Count tracked ticker configuration | `ForCountingTrackedTickers` | DuckDB |
| Resolve an exact underlying symbol | `ForResolvingUnderlyingSymbols` | Yahoo chart metadata |
| Obtain historical prices and corporate actions | `ForObtainingMarketHistory` | Yahoo |
| Obtain live prices | `ForObtainingLivePrices` | Yahoo |
| Stream live prices | `ForStreamingLivePrices` | Yahoo |
| Obtain option chains | `ForObtainingOptionChains` | CBOE |
| Obtain volatility-index history | `ForObtainingVolatilityIndices` | CBOE |
| Obtain risk-free yield curves | `ForObtainingYieldCurves` | U.S. Treasury |
| Consult exchange sessions | `ForConsultingTradingCalendar` | exchange calendar library |

The adapter column is configuration, not part of any port contract. Tests
replace each production adapter with an in-memory test double implementing the
same driven port.

Portfolio valuation is application orchestration rather than a driven actor.
It selects live or stored observations using the trading-calendar port, then
uses the specialized live-price, market-history, live-option, or stored-option
port. Consequently no adapter mixes Yahoo, CBOE, SQLite, and calendar logic.

SQLite and DuckDB are not selected dynamically by the application. The
production configurator selects DuckDB for every persistence port. Separate
offline driving conversations read SQLite only to migrate and verify legacy
data; SQLite remains an inactive, contract-tested proof of concept.

Provider aliases and wire formats are translated at the adapter boundary. For
example, the Yahoo adapter maps the domain ticker `SPX` to `^GSPC` and restores
`SPX` before returning the domain object. Neither alias is present in a port.

Sector performance reuses `ForLoadingMarketHistory` and derives 5-, 10-, or
21-session returns from `market_prices`; no derived sector table exists. The
benchmark is the tradable S&P 500 ETF `SPY`, not the `SPX` index, so benchmark
and sector ETFs share the same generic Yahoo history and persistence flow.
Initialization adds the eleven sector ETFs and `SPY` to tracked tickers with
historical prices enabled. Sector ETFs have option snapshots disabled, adding
eleven history requests but no sector-specific provider or pipeline (`SPY`
was already tracked).

The public periods `1w`, `2w`, and `1m` mean 5, 10, and 21 completed trading
session intervals respectively. A return uses the latest completed session at
or before `as_of` and the observation N sessions earlier, therefore requiring
N+1 valid observations. Its formula is `(end / start - 1) * 100`, preferring
adjusted close and falling back to close. Each sector must have observations
on the benchmark's exact start and end dates. Relative strength is the sector
return minus the `SPY` return, in percentage points.

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

Market-data refresh executions are coordinated by `ForRefreshingMarketData` and
persisted through `ForStoringDataRefreshRuns`/`ForLoadingDataRefreshRuns`. The
startup task, EOD scheduler, and manual HTTP request share that single use case.
Required system assets remain seeded in `tracked_tickers`; future user-added
underlyings must be distinguished there without creating another ticker list.
The bounded backfill policy applies only to refresh-eligible tracked tickers.
`active` remains the user's requested configuration. Exact underlying
resolution is a separate application conversation: user tickers are
`pending`, `resolved`, or `rejected`, while system tickers are `resolved` by
explicit bootstrap policy. `ForLoadingTrackedTickers` retains active-only
loading for management and separately exposes refresh eligibility. Refresh and
batch synchronization use only system tickers plus active, resolved user
tickers. Pending and rejected entries therefore cannot create refresh failures
or accelerate the global schedule.

The Yahoo resolver implements the provider-neutral
`ForResolvingUnderlyingSymbols` port. It returns only chart metadata actually
present in the response (currency, exchange, timezone, and instrument type).
It does not infer names, sectors, or option availability. Per-asset retry and
backoff are deliberately deferred; acquisition failures for already-resolved
assets still use the existing five-minute global retry policy.
Whether a valid underlying has listed options remains unknown in this phase;
requested `historical_prices` and `option_snapshots` are configuration, not
provider capabilities. Capability resolution belongs to phase 2.

Provider symbols remain inside the Yahoo adapter. The public identities
`BRK.B` and `SPX` are translated to `BRK-B` and `^GSPC` only for Yahoo calls and
are restored before crossing the driven port. Management and exact resolution
are independent driving ports, injected separately into the HTTP adapter even
when the composition root supplies the same application instance for both.
Tracked-ticker configuration uses monotonic coordination per normalized ticker:
the application records a new revision before provider access, does not hold a
lock during that access, and persists a response only if its revision is still
current. Independent tickers therefore do not block each other, and an older
provider response cannot create even a temporary eligibility window after a
newer deactivation or configuration.

The former `atualizar_dados.sh` was removed because it only downloaded SPY
option JSON in an independent one-minute loop and never participated in the
hexagonal scheduler or persistence lifecycle.

## Canonical public HTTP API and temporary aliases

Every canonical public HTTP operation is rooted at `/api`. The route inventory
and canonical-to-alias map are recorded in `docs/public-api.md`. Routes without
that prefix are compatibility aliases: they are registered against the same
handlers as their canonical counterparts and retain their status, content type,
and body. They may be removed only in a future, explicitly announced breaking
release, after consumers have migrated; this normalization does not set a
removal date.

The granular `/synchronization/*` operations are exceptional operational
aliases and are not canonical public API. They temporarily remain available for
compatibility, but no `/api/synchronization/*` routes exist. Startup, the EOD
scheduler, and manual refresh all use `ForRefreshingMarketData`. The only public
refresh operations are `GET /api/data-refresh/status` and
`POST /api/data-refresh`.

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
`target/doc/hexagonal_backend/index.html`. `target/`, SQLite files, and DuckDB files
under `data/` belong to this repository at runtime but are intentionally not
committed.
