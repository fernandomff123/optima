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

Plotly.js 3.0.1 is pinned by npm and copied into `dist` by Trunk. The Rust `plotly` crate supplies WASM-compatible models and bindings without embedding a second JavaScript payload in the WASM binary. No CDN or external runtime asset is used. The concrete adapter under `driving_adapters/ui/plotly` owns theme mapping and route-scoped `Plotly.purge` cleanup. Financial trace builders are deliberately deferred.

## Shell icons

The five inline shell icons are copied without redrawing from the official
[Lucide icon source](https://github.com/lucide-icons/lucide/tree/main/icons):
`layout-dashboard`, `chart-candlestick`, `search`, `briefcase`, and `settings`.
Lucide is distributed under the ISC license; `search` is derived from Feather
and retains its MIT license. Only these five local SVG definitions are compiled
into the application; there is no runtime icon dependency or external request.
