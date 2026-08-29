# Optima web foundation

This crate is a Leptos CSR application built by Trunk for `wasm32-unknown-unknown`.

```text
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
npm ci
npm run build:css
trunk build
```

Rust dependencies are pinned by the workspace `Cargo.lock`; npm dependencies are pinned by `package-lock.json`.

Plotly.js 3.0.1 is pinned by npm and copied into `dist` by Trunk. The Rust `plotly` crate supplies WASM-compatible models and bindings without embedding a second JavaScript payload in the WASM binary. No CDN or external runtime asset is used. The concrete adapter under `driving_adapters/ui/plotly` owns theme mapping and route-scoped `Plotly.purge` cleanup.

## Mocked Asset Overview

`/assets/:ticker/overview` is a mocks-first vertical slice. The Leptos page calls
`AssetOverviewUseCase`, which reads a provider-neutral snapshot through
`AssetOverviewPort`; `composition.rs` injects `MockAssetOverviewAdapter`. No HTTP
adapter or backend dependency is part of this slice.

The deterministic scenarios are selected with `?scenario=normal`, `loading`,
`stale`, `partial`, `unavailable`, `recoverable-error`, or `terminal-error`.
Unknown values fall back to `normal`. All timestamps and values are fixed visual
fixtures. In particular, IV Rank, IV Percentile, put-call ratios, Performance,
Index Facts, and Options Snapshot do not necessarily have backend contracts yet
and must not be treated as calculated or live financial data.

The optional Latest News panel is also a deterministic visual fixture. It has no
links or external requests and requires a dedicated backend contract before it
can become a live capability.

## Mocked Asset Options

`/assets/AAPL/options` is a separate mocks-first vertical slice based on the
active `Bloomberg/v2/assets-options.png` reference. The options page calls
`AssetOptionsUseCase` through `AssetOptionsPort`; `composition.rs` injects
`MockAssetOptionsAdapter`. The chain is an accessible HTML table and the IV
smile is rendered by the local Plotly adapter. No backend, HTTP adapter, DuckDB,
or external runtime asset is involved.

The fixed controls, option values, IV smile, and selected contract are visual
fixtures, not live prices or frontend financial calculations. Deterministic
states are available with `?scenario=loading`, `unavailable`, and
`recoverable-error`.

## Mocked Asset Chart

`/assets/AAPL/chart` follows the active candlestick workspace reference named
`ChatGPT Image Aug 26, 2026, 04_13_20 PM (1).png`. It loads provider-neutral
daily OHLC and volume fixtures through `AssetChartPort` and renders them with the
locally packaged Apache ECharts runtime. The adapter explicitly disconnects its
resize observer and disposes ECharts during route cleanup.

Use `?scenario=loading`, `unavailable`, or `recoverable-error` to inspect the
deterministic states. These values are visual fixtures; no HTTP, backend,
database, or external data source participates in the screen.

## Shell icons

The nine inline shell icons are copied without redrawing from the official
[Lucide icon source](https://github.com/lucide-icons/lucide/tree/main/icons):
`layout-dashboard`, `chart-candlestick`, `search`, `circle-dollar-sign`,
`activity`, `chart-no-axes-column`, `git-branch`, `briefcase`, and `settings`.
Lucide is distributed under the ISC license; `search` is derived from Feather
and retains its MIT license. Only these nine local SVG definitions are compiled
into the application; there is no runtime icon dependency or external request.
