# Option-analysis conversation

## Participants

| Role | Participant |
| --- | --- |
| Driving actor | Options analyst |
| Driving port | `ForAnalyzingOptions` |
| Coordinator | `OptionsApplication` |
| Driven actors | SQLite database and exchange calendar |

The driving port intentionally contains several related operations: retrieve an
option chain, build term structure, produce a volatility surface or skew, and
calculate Greeks. A port represents one coherent conversation, not necessarily
one function.

```text
Options analyst
  -> ForAnalyzingOptions
     -> ForLoadingOptionData             (SQLite)
     -> ForConsultingTradingCalendar     (when expiry timing is required)
     -> option-domain calculations
  <- chain / term structure / surface / skew / Greeks
```

Stored snapshots and reference data enter through `ForLoadingOptionData`.
Exchange-session timing enters separately through
`ForConsultingTradingCalendar`. The application supplies those values to pure
domain calculations; the domain does not read a database or calendar.

Obtaining a fresh Cboe chain is a different conversation, currently coordinated
by synchronization or intraday simulation through
`ForObtainingOptionChains`. This separation prevents “analyze stored options”
from silently performing external network I/O.
