use std::{fs, path::Path};
use walkdir::WalkDir;

fn rust_sources(root: &Path) -> Vec<(String, String)> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .map(|entry| {
            let path = entry.path().display().to_string();
            let source = fs::read_to_string(entry.path()).expect("Rust source must be readable");
            (path, source)
        })
        .collect()
}

#[test]
fn inner_layers_do_not_import_frameworks_or_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for layer in ["domain", "application"] {
        for (path, source) in rust_sources(&root.join(layer)) {
            for forbidden in [
                "leptos",
                "plotly",
                "driving_adapters",
                "driven_adapters",
                "gloo",
                "web_sys",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{path} imports forbidden dependency {forbidden}"
                );
            }
        }
    }
    assert!(!root.join("domain/presentation").exists());
}

#[test]
fn web_has_no_external_data_source_or_http_client() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, source) in rust_sources(&root.join("src")) {
        let lowercase = source.to_lowercase();
        for forbidden in [
            "yahoo", "cboe", "treasury", "http://", "https://", "gloo_net",
        ] {
            assert!(
                !lowercase.contains(forbidden),
                "{path} contains forbidden reference {forbidden}"
            );
        }
    }
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("manifest must be readable");
    for client in ["gloo-net", "reqwest"] {
        assert!(!manifest.contains(client));
    }

    for relative_path in ["index.html", "Trunk.toml", "styles/input.css"] {
        let path = root.join(relative_path);
        let source = fs::read_to_string(&path).expect("runtime input must be readable");
        for external in ["http://", "https://", "//cdn."] {
            assert!(
                !source.to_lowercase().contains(external),
                "{} contains external runtime resource {external}",
                path.display()
            );
        }
    }
}

#[test]
fn plotly_is_local_and_cleanup_uses_purge() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html = fs::read_to_string(root.join("index.html")).expect("index must be readable");
    let host = fs::read_to_string(root.join("src/driving_adapters/ui/plotly/host.rs"))
        .expect("Plotly host must be readable");
    assert!(html.contains("node_modules/plotly.js-dist-min/plotly.min.js"));
    assert!(html.contains("src=\"/plotly.min.js\""));
    assert!(host.contains("js_namespace = Plotly"));
    assert!(host.contains("purge(HOST_ID)"));
    let overview =
        fs::read_to_string(root.join("src/driving_adapters/ui/plotly/asset_overview.rs"))
            .expect("overview Plotly adapter must be readable");
    assert!(overview.contains("purge_plot(HOST_ID)"));
    assert!(overview.contains("build_price_volume_plot"));
    assert!(overview.contains("PriceVolumeChart"));
    assert!(!overview.contains("MockAssetOverviewAdapter"));
    assert!(overview.contains("displayModeBar:false"));
    assert!(overview.contains("displaylogo:false"));
    assert!(overview.contains("scrollZoom:false"));
    for preserved in ["Plotly.react", "responsive:true", "purgeOverviewPlot"] {
        assert!(overview.contains(preserved));
    }
    let traces = overview
        .split_once("const data = [")
        .and_then(|(_, source)| source.split_once("];"))
        .map(|(traces, _)| traces)
        .expect("Plotly trace array must be explicit");
    assert_eq!(traces.matches("{type:'").count(), 2);
    assert!(overview.contains("name:'Price', x:minutes, y:priceValues"));
    assert!(overview.contains("name:'Volume', x:minutes, y:Array.from(volumes)"));
    assert!(overview.contains("yaxis:'y2'"));
    assert!(!overview.contains("visible:false"));
    assert!(!overview.contains("modeBarButtons"));
    assert!(!overview.contains("toImage"));

    let css = fs::read_to_string(root.join("styles/input.css")).unwrap();
    assert!(!css.contains(".modebar"));
}

#[test]
fn asset_overview_respects_hexagonal_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let page = fs::read_to_string(root.join("driving_adapters/ui/pages/asset.rs")).unwrap();
    let application = fs::read_to_string(root.join("application/asset_overview/mod.rs")).unwrap();
    let composition = fs::read_to_string(root.join("composition.rs")).unwrap();
    assert!(page.contains("asset_overview_use_case"));
    assert!(!page.contains("MockAssetOverviewAdapter"));
    assert!(!page.contains("driven_adapters::mocks"));
    assert!(application.contains("AssetOverviewPort"));
    assert!(!application.contains("Plotly"));
    assert!(!application.contains("leptos"));
    assert!(composition.contains("MockAssetOverviewAdapter"));
}

#[test]
fn asset_overview_has_no_mock_badges_in_the_approved_header() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let header =
        fs::read_to_string(root.join("driving_adapters/ui/components/asset_header.rs")).unwrap();
    let page = fs::read_to_string(root.join("driving_adapters/ui/pages/asset.rs")).unwrap();
    assert_eq!(header.matches(">\"Mock\"<").count(), 0);
    assert!(!page.contains("badge=\"Mock\""));
    assert!(!header.contains("MockAssetOverviewAdapter"));
    assert!(!page.contains("MockAssetOverviewAdapter"));
}

#[test]
fn optional_content_and_fixtures_do_not_live_in_the_ui() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (path, source) in rust_sources(&root.join("driving_adapters/ui")) {
        for forbidden in [
            "Apple Services revenue reaches",
            "May 1, 2025",
            "AAPL_PRICES",
            "MockAssetOverviewAdapter",
        ] {
            assert!(
                !source.contains(forbidden),
                "{path} contains fixture {forbidden}"
            );
        }
    }
    let page = fs::read_to_string(root.join("driving_adapters/ui/pages/asset.rs")).unwrap();
    assert!(page.contains("model.earnings"));
    assert!(page.contains("model.index_facts"));
    assert!(!page.contains("model.symbol =="));
}

#[test]
fn financial_alignment_uses_explicit_tones_and_complete_values() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let value =
        fs::read_to_string(root.join("driving_adapters/ui/components/financial_value.rs")).unwrap();
    let table =
        fs::read_to_string(root.join("driving_adapters/ui/components/performance_table.rs"))
            .unwrap();
    let fixture = fs::read_to_string(root.join("driven_adapters/mocks/asset_overview.rs")).unwrap();
    assert!(!value.contains("grid-cols-"));
    assert!(value.contains("numeric inline-block whitespace-nowrap text-right"));
    assert!(value.contains("{value}{suffix}"));
    assert!(!value.contains("{unit.unwrap_or_default()}"));
    assert!(table.contains("table-header text-right"));
    assert!(!table.contains("starts_with"));
    assert!(!table.contains("contains('+')") && !table.contains("contains('-')"));
    assert!(fixture.contains("Total Open Interest"));
    assert!(fixture.contains("Some(\"contracts\")"));
    assert!(fixture.contains("Some(\"shares\")"));
    assert!(fixture.contains("Some(\"USD\")"));
    assert!(!value.contains("starts_with") && !value.contains("contains('%')"));
    let facts =
        fs::read_to_string(root.join("driving_adapters/ui/components/fact_table.rs")).unwrap();
    assert!(facts.contains("if metric.numeric"));
    assert!(!facts.contains("parse::<") && !facts.contains("starts_with"));
}

#[test]
fn lower_panels_keep_fixed_vertical_rhythm() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let page = fs::read_to_string(root.join("driving_adapters/ui/pages/asset.rs")).unwrap();
    let performance =
        fs::read_to_string(root.join("driving_adapters/ui/components/performance_table.rs"))
            .unwrap();
    let facts =
        fs::read_to_string(root.join("driving_adapters/ui/components/fact_table.rs")).unwrap();
    let news =
        fs::read_to_string(root.join("driving_adapters/ui/components/latest_news.rs")).unwrap();

    assert!(page.contains("2xl:h-[calc(100vh-13.125rem)]"));
    assert!(page.contains("2xl:min-h-[48.75rem]"));
    assert!(page.contains("2xl:grid-rows-[minmax(24.25rem,1fr)_minmax(23.5625rem,1fr)]"));
    assert!(page.contains("min-h-[24.25rem]"));
    assert!(page.contains("2xl:min-h-[23.5625rem]"));
    assert!(page.matches("2xl:h-full").count() >= 2);
    assert!(!performance.contains("h-full"));
    assert!(performance.contains("py-1.5"));
    assert!(!facts.contains("flex-1") && facts.contains("h-[2.375rem]"));
    assert!(!news.contains("flex-1") && news.contains("h-17"));
}

#[test]
fn global_sidebar_uses_local_lucide_icons_and_real_destinations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let navigation = fs::read_to_string(root.join("src/domain/navigation.rs")).unwrap();
    let icons =
        fs::read_to_string(root.join("src/driving_adapters/ui/components/icon.rs")).unwrap();
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    for (label, route) in [
        ("Options", "/options"),
        ("Volatility", "/volatility"),
        ("GEX / Flow", "/gex"),
        ("Simulations", "/simulations"),
    ] {
        assert!(navigation.contains(label) && navigation.contains(route));
    }
    for icon in ["Options", "Volatility", "Gex", "Simulations"] {
        assert!(icons.contains(&format!("ShellIconKind::{icon}")));
    }
    assert!(icons.contains("stroke=\"currentColor\"") && icons.contains("stroke-width=\"1.5\""));
    assert!(!manifest.contains("lucide") && !manifest.contains("icon"));
}

#[test]
fn overview_has_no_http_or_backend_contract_shortcuts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(!manifest.contains("gloo-net"));
    for (path, source) in rust_sources(&root.join("src")) {
        for forbidden in ["api_models", "reqwest", "duckdb"] {
            assert!(
                !source.to_lowercase().contains(forbidden),
                "{path} contains {forbidden}"
            );
        }
    }
}

#[test]
fn overview_route_and_query_scenarios_are_explicit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let router = fs::read_to_string(root.join("driving_adapters/ui/router.rs")).unwrap();
    let scenarios = fs::read_to_string(root.join("ports/asset_overview.rs")).unwrap();
    assert!(router.contains("assets/:ticker/overview"));
    for scenario in [
        "loading",
        "stale",
        "partial",
        "unavailable",
        "recoverable-error",
        "terminal-error",
    ] {
        assert!(scenarios.contains(scenario));
    }
}

#[test]
fn source_files_remain_reviewable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for (path, source) in rust_sources(&root) {
        let lines = source.lines().count();
        assert!(
            lines <= 350,
            "{path} has {lines} lines without justification"
        );
    }
}

#[test]
fn tailwind_contains_every_approved_hex_token() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let css =
        fs::read_to_string(root.join("styles/input.css")).expect("Tailwind input must be readable");
    for color in optima_web::design_system::tokens::APPROVED_HEX {
        assert!(
            css.contains(color),
            "Tailwind is missing approved token {color}"
        );
    }
}

#[test]
fn asset_overview_uses_only_approved_hex_colors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let approved = optima_web::design_system::tokens::APPROVED_HEX
        .into_iter()
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>();
    for relative in [
        "styles/input.css",
        "src/driving_adapters/ui/pages/asset.rs",
        "src/driving_adapters/ui/components/asset_header.rs",
        "src/driving_adapters/ui/components/asset_tabs.rs",
        "src/driving_adapters/ui/components/overview_metric.rs",
        "src/driving_adapters/ui/components/fact_table.rs",
        "src/driving_adapters/ui/components/performance_table.rs",
        "src/driving_adapters/ui/components/panel.rs",
        "src/driving_adapters/ui/components/key_statistics.rs",
        "src/driving_adapters/ui/components/latest_news.rs",
        "src/driving_adapters/ui/components/financial_value.rs",
        "src/driving_adapters/ui/plotly/asset_overview.rs",
    ] {
        let source = fs::read_to_string(root.join(relative)).unwrap();
        for token in source
            .split(|character: char| character.is_whitespace() || "();,\"'".contains(character))
        {
            if token
                .strip_prefix('#')
                .is_some_and(|hex| hex.len() == 6 && hex.chars().all(|ch| ch.is_ascii_hexdigit()))
            {
                assert!(
                    approved.contains(&token.to_ascii_uppercase()),
                    "{relative} contains unapproved color {token}"
                );
            }
        }
    }
}

#[test]
fn router_declares_the_approved_foundation_routes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let router = fs::read_to_string(root.join("src/driving_adapters/ui/router.rs"))
        .expect("router source must be readable");
    for route in [
        "markets",
        "assets",
        "assets/:ticker",
        "assets/:ticker/overview",
        "assets/:ticker/chart",
        "assets/:ticker/options",
        "assets/:ticker/volatility",
        "assets/:ticker/gex",
        "assets/:ticker/simulation",
        "portfolio",
        "settings",
    ] {
        let relative = format!("path!(\"{route}\")");
        let absolute = format!("path!(\"/{route}\")");
        assert!(
            router.contains(&relative) || router.contains(&absolute),
            "router is missing {route}"
        );
    }
}
