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

Apache ECharts 6.1.0 is also pinned locally for the Asset Chart workspace. Trunk
copies its minified browser bundle from `node_modules`; no CDN is used. The
route-scoped adapter under `driving_adapters/ui/echarts` owns initialization,
resize, render, and disposal. Indicator calculations remain behind
`TechnicalIndicatorPort` and do not depend on ECharts types.

## Mocked Asset Chart

`/assets/AAPL/chart` is a deterministic, full-viewport technical-analysis
workspace. The page loads OHLCV fixtures through `AssetChartPort`, calculates
MA 20/50/200, RSI 14, and MACD through the YATA adapter, and sends only the
provider-neutral read model to the route-scoped ECharts renderer. Indicator
visibility can be changed locally from the right-hand panel.

The chart deliberately omits Call Wall, Put Wall, and Gamma Flip levels. Those
labels require explicit backend definitions and must not be inferred from mock
OHLCV data. Compare and drawing controls remain visibly unavailable until their
interactions have real contracts. Loading, unavailable, and recoverable-error
states are available through the same `?scenario=` values used by other slices.

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

## Shell icons

The nine inline shell icons are copied without redrawing from the official
[Lucide icon source](https://github.com/lucide-icons/lucide/tree/main/icons):
`layout-dashboard`, `chart-candlestick`, `search`, `circle-dollar-sign`,
`activity`, `chart-no-axes-column`, `git-branch`, `briefcase`, and `settings`.
Lucide is distributed under the ISC license; `search` is derived from Feather
and retains its MIT license. Only these nine local SVG definitions are compiled
into the application; there is no runtime icon dependency or external request.
