# Public API route inventory

This is the mechanical inventory captured before normalization. `domain` means
the old modern handler accepted or serialized a hexagon type directly. The
known consumer is based on the adapter's existing compatibility contract and
repository tests; no frontend source was inspected for this change.

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
