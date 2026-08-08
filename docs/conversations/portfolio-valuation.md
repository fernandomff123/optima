# Portfolio valuation conversation

## Participants

| Role | Participant |
| --- | --- |
| Driving actor | Portfolio viewer |
| Driving port | `ForViewingPortfolioPositions` |
| Coordinator | `PortfolioValuationApplication` |
| Driven actors | DuckDB, exchange calendar, Yahoo Finance, Cboe |

## Coordination

```text
Portfolio viewer
  -> ForViewingPortfolioPositions.valued_positions
     -> ForLoadingPortfolios                    (DuckDB)
     -> ForConsultingTradingCalendar            (exchange calendar)
     -> for every position
        -> equity + open session
           -> ForObtainingLivePrices            (Yahoo)
        -> equity + closed session
           -> ForLoadingMarketHistory           (DuckDB)
        -> option + open session
           -> ForObtainingOptionChains          (Cboe)
        -> option + closed session
           -> ForLoadingOptionChains            (DuckDB)
     -> domain quantity x price x contract multiplier
  <- valued positions
```

The application parses the technology-neutral OCC symbol, maps an option to its
underlying chain, caches a chain used by several positions, and decides between
live and stored observations. These are use-case decisions, so they do not
belong to a Yahoo, Cboe, DuckDB, or calendar adapter.

A missing market observation leaves that position unvalued without hiding the
rest of the portfolio. A missing portfolio fails the whole conversation.

## Test-to-test

`tests/portfolio_valuation_ports.rs` drives the real application through the
driving port while replacing calendar, prices, history, option chain, storage,
and portfolio persistence actors with test doubles. This verifies coordination
without HTTP, provider access, or a database.
