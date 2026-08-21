# Public API route inventory

This is the mechanical inventory captured before normalization. `domain` means
the old modern handler accepted or serialized a hexagon type directly. The
known consumer is based on the adapter's existing compatibility contract and
repository tests. The frontend source was inspected for the nullable option
market facts described below and does not currently consume them.

## Nullable option market facts

Option contracts returned by `GET /api/options/{ticker}/chain`, its
`GET /options/{ticker}/chain` compatibility alias,
`GET /api/simulation/contracts`, and
`GET /api/assets/{ticker}/options/intraday` preserve `gamma` and
`open_interest` factually:

- a JSON number is the value published in the snapshot;
- `0.0` is a published zero and is distinct from absence;
- JSON `null` means that the value is unavailable in that snapshot.

Absence of either field never removes its contract from these responses.

## Modern router before normalization

| Method | Existing path | Handler | Request → response | Domain crossed HTTP | Known consumer | Canonical path |
| --- | --- | --- | --- | --- | --- | --- |
| GET | `/market-data/{ticker}/history` | `market_history` | path → market history | response | tests/tools | `/api/market-data/{ticker}/history` |
| GET | `/market-data/{ticker}/live-price` | `live_price` | path → live price | response | tests/tools | `/api/market-data/{ticker}/live-price` |
| GET | `/options/{ticker}/chain` | `option_chain` | path → snapshot | response | tests/tools | `/api/options/{ticker}/chain` |
| GET | `/options/{ticker}/term-structure` | `term_structure` | path → term structure | response | tests/tools | `/api/options/{ticker}/term-structure` |
| GET | `/options/{ticker}/surface` | `volatility_surface` | path → surface | response | tests/tools | `/api/options/{ticker}/surface` |
| GET | `/options/{ticker}/skew/{expiration}` | `volatility_skew` | path → skew | response | tests/tools | `/api/options/{ticker}/skew/{expiration}` |
| GET | `/options/{ticker}/contracts/{occ_symbol}/greeks` | `greeks` | path → greeks | response | tests/tools | `/api/options/{ticker}/contracts/{occ_symbol}/greeks` |
| POST | `/simulation/grid` | `build_scenario_grid` | grid request → grid | both | tests/tools | `/api/strategy-simulation/grid` |
| POST | `/simulation` | `simulate_strategy` | simulation request → result | both | tests/tools | `/api/strategy-simulation` |
| POST | `/portfolios` | `create_portfolio` | create request → empty 201 | request fields local | tests/tools | `/api/portfolios` |
| POST | `/portfolios/{id}/cash-movements` | `record_cash_movement` | cash movement → empty 204 | request | tests/tools | `/api/portfolios/{id}/cash-movements` |
| POST | `/portfolios/{id}/option-trades` | `record_option_trade` | trade → empty 204 | request | tests/tools | `/api/portfolios/{id}/option-trades` |
| POST | `/portfolios/{id}/currency-exchanges` | `record_currency_exchange` | exchange → empty 204 | request | tests/tools | `/api/portfolios/{id}/currency-exchanges` |
| GET | `/portfolios/{id}/balance` | `check_balance` | path → currency map | no | tests/tools | `/api/portfolios/{id}/balance` |
| GET | `/portfolios/{id}/positions` | `list_positions` | path → positions | response | tests/tools | `/api/portfolios/{id}/positions` |
| GET | `/portfolios/{id}/transactions` | `list_transactions` | path → events | response | tests/tools | `/api/portfolios/{id}/transactions` |
| POST | `/synchronization/market-history/{ticker}` | `synchronize_market_history` | since → report | port types | operational tools | none (non-canonical) |
| POST | `/synchronization/tracked-tickers` | `synchronize_tracked_tickers` | sync window → report | port types | operational tools | none (non-canonical) |
| POST | `/synchronization/option-chain/{ticker}` | `synchronize_option_chain` | close → report | port types | operational tools | none (non-canonical) |
| POST | `/synchronization/term-structure/{ticker}` | `synchronize_term_structure` | path → report | port response | operational tools | none (non-canonical) |
| POST | `/synchronization/volatility-index/{ticker}` | `synchronize_volatility_index` | path → report | port response | operational tools | none (non-canonical) |
| POST | `/synchronization/yield-curves/{year}` | `synchronize_yield_curves` | path → report | port response | operational tools | none (non-canonical) |
| GET | `/saved-strategies` | `list_saved_strategies` | empty → strategies | response | tests/tools | `/api/saved-strategies` |
| POST | `/saved-strategies` | `save_strategy` | save command → strategy | both | tests/tools | `/api/saved-strategies` |
| DELETE | `/saved-strategies/{id}` | `delete_strategy` | path → empty 204 | no | tests/tools | `/api/saved-strategies/{id}` |
| GET | `/tracked-tickers` | `list_tracked_tickers` | empty → tickers | response | tests/tools | `/api/tracked-tickers` |
| PUT | `/tracked-tickers/{ticker}` | `configure_tracked_ticker` | configuration → empty 204 | constructed domain | tests/tools | `/api/tracked-tickers/{ticker}` |
| GET | `/api/underlyings/resolve?ticker={ticker}` | `resolve_underlying` | exact ticker → factual resolution | mapped | frontend/tests | unchanged canonical; no alias |
| GET | `/api/market/sectors` | `view_sector_performance` | period query → sector response | mapped | frontend/tests | unchanged canonical |
| GET | `/api/data-refresh/status` | `data_refresh_status` | empty → refresh status | mapped | frontend/manual | unchanged canonical |
| POST | `/api/data-refresh` | `request_data_refresh` | empty → refresh result | mapped | frontend/manual | unchanged canonical |

## Compatibility router before and after normalization

These existing `/api` contracts were already canonical and remain unchanged:

| Operations | Paths | Handler family | Request/response | Known consumer |
| --- | --- | --- | --- | --- |
| GET | `/api/health`, `/api/assets`, `/api/market/{benchmark,volatility,spx-history,vix-history,rates}` | health/market views | `api_models` (health is text) | frontend |
| GET/WS | `/api/assets/live` | `asset_live_prices` | WebSocket `api_models` messages | frontend |
| GET+POST, GET×4 | `/api/portfolio`, `/api/portfolio/{summary,cash,positions,movements}` | portfolio views/commands | `api_models` | frontend |
| GET+POST, POST, GET | `/api/simulation`, `/api/simulation/intraday`, `/api/simulation/contracts` | simulation views | `api_models` | frontend |
| GET+POST, DELETE | `/api/strategies`, `/api/strategies/{id}` | saved strategy views | `api_models` | frontend |
| POST×2 | `/api/portfolio/option-trades`, `/api/portfolio/currency-exchanges` | portfolio commands | `api_models` | frontend |
| GET×8 | `/api/assets/{ticker}/{price,price-history,historical-volatility,implied-volatility,options/snapshot,options/term-structure,options/volatility-surface,options/intraday}` | asset/option views | `api_models` | frontend |

## After normalization

The 57 route declarations that existed before this work remain registered (some
declarations serve more than one HTTP method). Twenty-one canonical
path+method operations were added and map to the aliases in the first table.
There are no new granular synchronization routes. Public modern request and
response bodies now use explicit `api_models` DTOs and adapter-owned mappings;
their serialized legacy representation is preserved.

Pre-existing behavior intentionally retained includes SPX history returning 500
without VIX and simulation returning 404 when required market data is absent.
Shutdown lifecycle, underlying invariants, DuckDB concurrency, persistence, and
business rules are outside this normalization.

## Tracked ticker management

`GET /api/tracked-tickers` preserves the historical behavior and returns only
active entries. `GET /api/tracked-tickers?include_inactive=true` returns the
complete catalog. Each entry contains `ticker`, `source` (`system` or `user`),
`active`, `historical_prices`, `option_snapshots`, `resolution_state`,
`validated_at`, and factual `metadata` (`currency`, `exchange`, `timezone`, and
`instrument_type`). The temporary
`GET /tracked-tickers` alias accepts the same query and returns the same status,
`application/json` content type, and body.

`PUT /api/tracked-tickers/{ticker}` accepts `active`, `historical_prices`, and
`option_snapshots`, and returns `204 No Content`. It creates a user ticker when
none exists (absence is otherwise represented by omission from the list) and
otherwise replaces that user ticker's configuration. Repeating the same
request is idempotent. A new user ticker, or a pending user ticker being
activated, is resolved factually before it becomes refresh-eligible. Conclusive absence returns `404`;
transient provider failure returns `503`. Neither outcome creates a new
eligible ticker. Existing resolved tickers can be updated or disabled without
provider access, and an existing ticker can always be disabled without a
resolution attempt. Tickers are trimmed and uppercased in the domain. Invalid
symbols return `400` with the canonical `ApiError` envelope;
an identical configuration of a protected system ticker is idempotent, while
an attempted change returns `409`; persistence
failures return `503`. The temporary `PUT /tracked-tickers/{ticker}` alias uses
the same handler and preserves the successful `204` response with no body and
no content type.

There is deliberately no delete endpoint. Deactivation retains history,
option snapshots, portfolio events, and saved-strategy references. SPX, SPY,
VIX, XLB, XLC, XLE, XLF, XLI, XLK, XLP, XLRE, XLU, XLV, and XLY are active
system entries and cannot be configured through this API.

## Exact underlying resolution

`GET /api/underlyings/resolve?ticker=MSFT` validates the ticker syntax and asks
the exact-resolution application conversation to confirm it through the
configured provider. Success returns `200 application/json` with `ticker`,
`validated_at`, and factual `metadata`. The operation neither persists nor
activates the ticker. Invalid syntax returns `400`, conclusive absence returns
`404`, and transient or incompatible provider responses return `503`, all with
the canonical `ApiError` envelope. There is intentionally no non-`/api` alias.

Resolution confirms identity only. It does not claim historical-data or option
capabilities, and it does not provide name, sector search, or autocomplete.
In particular, a valid ticker with no listed options is not yet classified by
this phase; `historical_prices` and `option_snapshots` remain user requests.
