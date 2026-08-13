# Market-data synchronization conversation

## Participants

| Role | Participant |
| --- | --- |
| Driving actor | Data operator or scheduler |
| Driving port | `ForSynchronizingMarketData` |
| Coordinator | `SynchronizationApplication` |
| Provider actors | Yahoo Finance, Cboe, U.S. Treasury |
| Supporting actors | DuckDB database, exchange calendar |

## Sub-conversations

| Operation | Acquisition port | Persistence port | Additional coordination |
| --- | --- | --- | --- |
| Tracked tickers | `ForLoadingTrackedTickers` plus the appropriate provider port | Domain-specific store port | Iterates only configured active instruments |
| Market history | `ForObtainingMarketHistory` | `ForStoringMarketHistory` | Applies the requested start date |
| Option chain | `ForObtainingOptionChains` | `ForStoringOptionChains` | Normalizes contracts in DuckDB and associates them with an eligible market close |
| Term structure | Stored volatility analytics | `ForStoringVolatilityTermStructures` | Uses stored yield curves and exchange-session timing for domain calculation |
| Volatility index | `ForObtainingVolatilityIndices` | `ForStoringIndexHistory` | Keeps index history separate from equity history |
| Yield curves | `ForObtainingYieldCurves` | `ForStoringYieldCurves` | Synchronizes the requested publication year |

## Coordination

```text
Operator/scheduler
  -> ForSynchronizingMarketData
     -> choose requested synchronization operation
     -> obtain external observation through a provider-neutral port
     -> calculate/validate domain values when required
     -> store through the matching domain persistence port
  <- SynchronizationReport
```

The coordinator, not a provider adapter, owns the obtain-then-store workflow.
Consequently Yahoo never writes DuckDB, Cboe never invokes the calendar, and a
database adapter never downloads data. A different implementation can be chosen
for any one port in the configurator without rewriting the use case.

`SynchronizationStores` and `OptionAnalysisCollaborators` are constructor
parameter groupings. They do not implement ports and do not hide technology;
their fields remain independently constrained by the required port traits.

The tracked-ticker catalog is the single selection source for batch and
end-of-day refresh. Active system and user entries follow the same loop;
`historical_prices` and `option_snapshots` independently select its two market
data capabilities. VIX index history remains the dedicated volatility-index
step and therefore has both generic capabilities disabled in the catalog.
