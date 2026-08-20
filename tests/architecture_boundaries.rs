use std::{fs, path::Path};

fn rust_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("architecture directory must exist") {
        let path = entry.expect("directory entry must be readable").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn driving_binaries_do_not_select_concrete_driven_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin");
    let mut files = Vec::new();
    rust_files(&root, &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("binary source must be readable");
        assert!(
            !source.contains("driven_adapters::"),
            "{} selects a concrete driven adapter outside the configurator",
            path.display()
        );
    }
}

#[test]
fn hexagon_does_not_depend_on_external_actors_or_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hexagon");
    let forbidden = [
        "axum",
        "reqwest",
        "sqlx",
        "sqlite",
        "yahoo",
        "cboe",
        "treasury.gov",
        "driven_adapters",
        "driving_adapters",
        "storage::",
        "api::",
        "parser::",
    ];
    let mut files = Vec::new();
    rust_files(&root, &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("Rust source must be readable");
        for forbidden_name in forbidden {
            assert!(
                !source.to_ascii_lowercase().contains(forbidden_name),
                "{} contains forbidden dependency/name '{forbidden_name}'",
                path.display()
            );
        }
    }
}

#[test]
fn production_code_does_not_use_panicking_value_extraction() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&root, &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("Rust source must be readable");
        // Unit-test modules are conventionally the final cfg(test) section of
        // each source file and may use assertions with explicit fixtures.
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for forbidden in [".unwrap(", ".expect("] {
            assert!(
                !production.contains(forbidden),
                "{} uses panicking extraction '{forbidden}' in production code",
                path.display()
            );
        }
    }
}

#[test]
fn web_server_has_one_composition_and_one_http_router() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let server = fs::read_to_string(root.join("src/bin/web_server.rs"))
        .expect("web server source must be readable");
    assert_eq!(server.matches("configurator::configure()").count(), 1);
    assert_eq!(
        server.matches("configure_server_http_application(").count(),
        1
    );
    assert!(!server.contains("Router::new()"));
    assert!(!server.contains(".route("));
    assert!(server.contains("let data_refresh = configured.data_refresh.clone();"));
    assert!(server.contains("run_startup_refresh(data_refresh.clone())"));
    assert!(server.contains("run_market_eod_scheduler(data_refresh)"));
    assert!(server.contains("configure_server_http_application(\n        configured,"));
}

#[test]
fn http_handlers_only_receive_injected_ports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/driving_adapters/http");
    let mut files = Vec::new();
    rust_files(&root, &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("HTTP adapter source must be readable");
        assert!(
            !source.contains("configurator::configure"),
            "{} calls the composition root from an HTTP adapter",
            path.display()
        );
        assert!(
            !source.contains("driven_adapters::"),
            "{} constructs or calls a driven adapter",
            path.display()
        );
    }
}

#[test]
fn websocket_receives_market_ports_by_injection() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/driving_adapters/http/legacy_server.rs"))
        .expect("legacy HTTP adapter source must be readable");
    assert!(source.contains("market_data: Arc<dyn ForViewingMarketData>"));
    assert!(source.contains("market_stream: Arc<dyn ForStreamingMarketPrices>"));
    assert!(source.contains("async fn handle_asset_live_prices("));
    assert!(source.contains("state.market_data,"));
    assert!(source.contains("state.market_stream,"));
}

#[test]
fn applications_do_not_know_axum() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hexagon/application");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    for path in files {
        let source = fs::read_to_string(&path).expect("application source must be readable");
        assert!(
            !source.to_ascii_lowercase().contains("axum"),
            "{} depends on Axum",
            path.display()
        );
    }
}

#[test]
fn tracked_ticker_policy_stays_inside_the_hexagon() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adapter = fs::read_to_string(root.join("src/driven_adapters/duckdb/tracked_tickers.rs"))
        .expect("tracked ticker adapter source must be readable");
    let application =
        fs::read_to_string(root.join("src/hexagon/application/tracked_tickers/mod.rs"))
            .expect("tracked ticker application source must be readable");
    let domain = fs::read_to_string(root.join("src/hexagon/domain/tracked_ticker.rs"))
        .expect("tracked ticker domain source must be readable");

    assert!(!adapter.contains("SPX"));
    assert!(!adapter.contains("system_tickers"));
    assert!(application.contains("is_system_ticker"));
    assert!(domain.contains("super::sector_performance::{SECTOR_BENCHMARK_TICKER, SECTORS}"));
    assert!(!domain.contains("\"XLK\""));

    let configurator = fs::read_to_string(root.join("src/configurator/mod.rs"))
        .expect("configurator source must be readable");
    assert!(configurator.contains(".bootstrap_system_tickers()"));
    assert!(!configurator.contains("domain::tracked_ticker::system_tickers"));
}

#[test]
fn tracked_ticker_loading_operations_have_no_semantic_fallback() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let port = fs::read_to_string(
        root.join("src/hexagon/driven_ports/for_loading_tracked_tickers/mod.rs"),
    )
    .expect("tracked ticker loading port must be readable");

    assert!(
        port.contains("async fn load_tracked_tickers(&self) -> PortResult<Vec<TrackedTicker>>;")
    );
    assert!(
        port.contains("async fn load_active_tickers(&self) -> PortResult<Vec<TrackedTicker>>;")
    );
    assert!(port.contains(
        "async fn load_refresh_eligible_tickers(&self) -> PortResult<Vec<TrackedTicker>>;"
    ));
    assert!(!port.contains("self.load_active_tickers().await"));
}

#[test]
fn underlying_resolution_remains_a_hexagonal_conversation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domain = fs::read_to_string(root.join("src/hexagon/domain/tracked_ticker.rs")).unwrap();
    let application =
        fs::read_to_string(root.join("src/hexagon/application/tracked_tickers/mod.rs")).unwrap();
    let http = fs::read_to_string(root.join("src/driving_adapters/http/mod.rs")).unwrap();
    let management_port = fs::read_to_string(
        root.join("src/hexagon/driving_ports/for_managing_tracked_tickers/mod.rs"),
    )
    .unwrap();
    let resolution_port =
        fs::read_to_string(root.join("src/hexagon/driving_ports/for_resolving_underlyings/mod.rs"))
            .unwrap();
    let refresh =
        fs::read_to_string(root.join("src/hexagon/application/data_refresh/mod.rs")).unwrap();
    let synchronization =
        fs::read_to_string(root.join("src/hexagon/application/synchronization/mod.rs")).unwrap();

    assert!(!domain.contains("Yahoo"));
    assert!(!domain.contains("reqwest"));
    assert!(application.contains("ForResolvingUnderlyingSymbols"));
    assert!(application.contains("ForResolvingUnderlyings"));
    assert!(!management_port.contains("ForResolvingUnderlyings"));
    assert!(management_port.contains("pub trait ForManagingTrackedTickers: Send + Sync"));
    assert!(resolution_port.contains("pub trait ForResolvingUnderlyings: Send + Sync"));
    assert!(http.contains("tracked_tickers: Arc<dyn ForManagingTrackedTickers>"));
    assert!(http.contains("underlying_resolver: Arc<dyn ForResolvingUnderlyings>"));
    assert!(http.contains(".underlying_resolver\n        .resolve_underlying"));
    assert!(http.contains(".tracked_tickers\n        .configure_ticker"));
    assert!(!http.contains("YahooUnderlyingResolverAdapter"));
    assert!(!http.contains("driven_adapters::yahoo"));
    assert!(refresh.contains("load_refresh_eligible_tickers()"));
    assert!(!refresh.contains("load_active_tickers()"));
    assert!(synchronization.contains("load_refresh_eligible_tickers()"));
    assert!(!synchronization.contains("load_active_tickers()"));
}

#[test]
fn all_legacy_route_patterns_remain_in_the_http_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/driving_adapters/http/legacy_server.rs"))
        .expect("legacy HTTP adapter source must be readable");
    let expected = [
        "/api/health",
        "/api/assets",
        "/api/assets/live",
        "/api/market/benchmark",
        "/api/market/volatility",
        "/api/market/spx-history",
        "/api/market/vix-history",
        "/api/market/rates",
        "/api/portfolio",
        "/api/portfolio/summary",
        "/api/portfolio/cash",
        "/api/portfolio/positions",
        "/api/portfolio/movements",
        "/api/simulation",
        "/api/simulation/intraday",
        "/api/strategies",
        "/api/strategies/{id}",
        "/api/simulation/contracts",
        "/api/portfolio/option-trades",
        "/api/portfolio/currency-exchanges",
        "/api/assets/{ticker}/price",
        "/api/assets/{ticker}/price-history",
        "/api/assets/{ticker}/historical-volatility",
        "/api/assets/{ticker}/implied-volatility",
        "/api/assets/{ticker}/options/snapshot",
        "/api/assets/{ticker}/options/term-structure",
        "/api/assets/{ticker}/options/volatility-surface",
        "/api/assets/{ticker}/options/intraday",
    ];
    for route in expected {
        assert!(source.contains(route), "legacy route disappeared: {route}");
    }
}
