# Intraday market conversations

Two related conversations illustrate different amounts of application
coordination.

## Prepare an intraday option market

| Role | Participant |
| --- | --- |
| Driving actor | Intraday analyst or simulation client |
| Driving ports | `ForViewingIntradayOptions`, `ForPreparingIntradaySimulations` |
| Coordinator | `IntradaySimulationApplication` |
| Driven actors | Cboe, Yahoo Finance, exchange calendar |

```text
Intraday actor
  -> intraday driving port
     -> ForConsultingTradingCalendar
     -> ForObtainingOptionChains         (Cboe)
     -> ForObtainingLivePrices           (Yahoo)
     -> combine technology-neutral observations
  <- IntradaySimulationMarket
```

The application decides whether the request is valid for the regular session
and combines option and underlying-price observations. Neither provider adapter
knows or calls the other.

## Stream live prices

| Role | Participant |
| --- | --- |
| Driving actor | Live-price subscriber |
| Driving port | `ForStreamingMarketPrices` |
| Coordinator | `MarketStreamApplication` |
| Driven actor | Yahoo Finance |

This use case needs no cross-actor decision, so the application delegates the
stream through `ForStreamingLivePrices`. The application boundary still matters:
the driving side consumes `LivePrice`, not Yahoo WebSocket frames, symbols, or
protobuf messages. A future provider adapter can replace Yahoo without changing
the driving port.
