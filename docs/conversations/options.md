# Option-analysis conversation

## Participants

| Role | Participant |
| --- | --- |
| Driving actor | Options analyst |
| Driving port | `ForAnalyzingOptions` |
| Coordinator | `OptionsApplication` |
| Driven actors | DuckDB and exchange calendar |

The driving port intentionally contains several related operations: retrieve an
option chain, build term structure, produce a volatility surface or skew, and
calculate Greeks. A port represents one coherent conversation, not necessarily
one function.

```text
Options analyst
  -> ForAnalyzingOptions
     -> ForLoadingOptionChains           (DuckDB)
     -> ForLoadingVolatilityTermStructures (DuckDB)
     -> ForLoadingReferencePrices          (DuckDB)
     -> ForLoadingYieldCurves               (DuckDB)
     -> ForConsultingTradingCalendar     (when expiry timing is required)
     -> option-domain calculations
  <- chain / term structure / surface / skew / Greeks
```

Stored snapshots enter through `ForLoadingOptionChains`; the production adapter
is DuckDB. Reference
prices, derived volatility structures, and yield curves enter through separate
technology-neutral ports implemented by DuckDB production adapters.
Exchange-session timing enters separately through
`ForConsultingTradingCalendar`. The application supplies those values to pure
domain calculations; the domain does not read a database or calendar.

Obtaining a fresh Cboe chain is a different conversation, currently coordinated
by synchronization or intraday simulation through
`ForObtainingOptionChains`. This separation prevents “analyze stored options”
from silently performing external network I/O.
