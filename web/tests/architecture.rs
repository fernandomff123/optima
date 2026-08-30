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
fn echarts_is_local_route_scoped_and_only_a_driving_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html = fs::read_to_string(root.join("index.html")).unwrap();
    let package = fs::read_to_string(root.join("package.json")).unwrap();
    let runtime =
        fs::read_to_string(root.join("src/driving_adapters/ui/echarts/runtime.rs")).unwrap();
    let host = fs::read_to_string(root.join("src/driving_adapters/ui/echarts/host.rs")).unwrap();

    assert!(package.contains("\"echarts\": \"6.1.0\""));
    assert!(html.contains("node_modules/echarts/dist/echarts.min.js"));
    assert!(html.contains("src=\"/echarts.min.js\""));
    assert!(!html.contains("cdn"));
    assert!(runtime.contains("globalThis.echarts.init"));
    assert!(runtime.contains("chart.setOption"));
    assert!(runtime.contains("chart.resize"));
    assert!(runtime.contains("chart.dispose"));
    assert!(host.contains("on_cleanup"));
    assert!(host.contains("dispose_chart"));

    for layer in ["domain", "application", "ports"] {
        for (path, source) in rust_sources(&root.join("src").join(layer)) {
            assert!(
                !source.to_lowercase().contains("echarts"),
                "{path} couples an inner layer to ECharts"
            );
        }
    }
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
fn asset_options_respects_hexagonal_boundaries_and_keeps_fixtures_in_mock_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let page = fs::read_to_string(root.join("driving_adapters/ui/pages/asset_options.rs")).unwrap();
    let application = fs::read_to_string(root.join("application/asset_options/mod.rs")).unwrap();
    let mock = fs::read_to_string(root.join("driven_adapters/mocks/asset_options.rs")).unwrap();
    let composition = fs::read_to_string(root.join("composition.rs")).unwrap();
    assert!(page.contains("asset_options_use_case"));
    assert!(!page.contains("MockAssetOptionsAdapter"));
    assert!(!page.contains("191.13"));
    assert!(application.contains("AssetOptionsPort"));
    assert!(!application.contains("Plotly"));
    assert!(!application.contains("leptos"));
    assert!(mock.contains("191.13"));
    assert!(mock.contains("MockAssetOptionsAdapter"));
    assert!(composition.contains("MockAssetOptionsAdapter"));
}

#[test]
fn yata_is_isolated_behind_the_technical_indicator_port() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let application =
        fs::read_to_string(root.join("application/technical_indicators/mod.rs")).unwrap();
    let port = fs::read_to_string(root.join("ports/technical_indicators.rs")).unwrap();
    let adapter =
        fs::read_to_string(root.join("driven_adapters/technical_indicators/yata.rs")).unwrap();

    assert!(application.contains("TechnicalIndicatorPort"));
    assert!(!application.contains("yata::"));
    assert!(!port.contains("yata::"));
    assert!(adapter.contains("YataTechnicalIndicatorAdapter"));
    assert!(adapter.contains("yata::"));
    assert!(adapter.contains("BollingerBands"));
    assert!(adapter.contains("RelativeStrengthIndex"));
    assert!(adapter.contains("MACD"));
    assert!(adapter.contains("SMA"));
}

#[test]
fn asset_chart_keeps_fixtures_calculation_and_rendering_in_separate_adapters() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let page = fs::read_to_string(root.join("driving_adapters/ui/pages/asset_chart.rs")).unwrap();
    let renderer =
        fs::read_to_string(root.join("driving_adapters/ui/echarts/asset_chart.rs")).unwrap();
    let catalog =
        fs::read_to_string(root.join("driving_adapters/ui/components/chart_indicator_catalog.rs"))
            .unwrap();
    let simulation =
        fs::read_to_string(root.join("driving_adapters/ui/components/chart_simulation_action.rs"))
            .unwrap();
    let application = fs::read_to_string(root.join("application/asset_chart/mod.rs")).unwrap();
    let mock = fs::read_to_string(root.join("driven_adapters/mocks/asset_chart.rs")).unwrap();
    let composition = fs::read_to_string(root.join("composition.rs")).unwrap();

    assert!(page.contains("asset_chart_use_case"));
    assert!(page.contains("IndicatorToggle"));
    assert!(page.contains("ChartIndicatorCatalog"));
    assert!(!page.contains("MockAssetChartAdapter"));
    assert!(!page.contains("YataTechnicalIndicatorAdapter"));
    assert!(application.contains("AssetChartPort"));
    assert!(application.contains("TechnicalIndicatorsUseCase"));
    assert!(!application.contains("echarts"));
    assert!(!application.contains("leptos"));
    assert!(mock.contains("55_210_000.0"));
    assert!(renderer.contains("candlestick"));
    assert!(renderer.contains("dataZoom"));
    assert!(renderer.contains("xAxisIndex"));
    assert!(!renderer.contains("MockAssetChartAdapter"));
    assert!(composition.contains("MockAssetChartAdapter"));
    assert!(composition.contains("YataTechnicalIndicatorAdapter"));
    assert!(mock.contains("GexLevelSnapshot"));
    assert!(mock.contains("Call Wall"));
    assert!(mock.contains("Gamma Flip"));
    assert!(mock.contains("Put Wall"));
    assert!(page.contains("Mock fixture"));
    assert!(page.contains("set_gex"));
    assert!(renderer.contains("visibility.gex"));
    assert!(catalog.contains("aria-modal=\"true\""));
    assert!(catalog.contains(">\"Done\"<"));
    assert!(simulation.contains("Add underlying to Simulation"));
    assert!(simulation.contains("100_u32"));
    assert!(simulation.contains("step=\"100\""));
    assert!(simulation.contains("\"Long\"") && simulation.contains("\"Short\""));
    assert!(!simulation.contains("http"));
}

#[test]
fn asset_simulation_keeps_financial_fixtures_behind_its_port() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let application = fs::read_to_string(root.join("application/asset_simulation/mod.rs")).unwrap();
    let port = fs::read_to_string(root.join("ports/asset_simulation.rs")).unwrap();
    let mock = fs::read_to_string(root.join("driven_adapters/mocks/asset_simulation.rs")).unwrap();
    let composition = fs::read_to_string(root.join("composition.rs")).unwrap();
    let page =
        fs::read_to_string(root.join("driving_adapters/ui/pages/asset_simulation.rs")).unwrap();
    let payoff =
        fs::read_to_string(root.join("driving_adapters/ui/echarts/asset_simulation.rs")).unwrap();
    let heatmap =
        fs::read_to_string(root.join("driving_adapters/ui/components/simulation_heatmap.rs"))
            .unwrap();
    let scenario =
        fs::read_to_string(root.join("driving_adapters/ui/components/simulation_scenario.rs"))
            .unwrap();
    let position =
        fs::read_to_string(root.join("driving_adapters/ui/components/simulation_position.rs"))
            .unwrap();
    let draft = fs::read_to_string(root.join("driving_adapters/ui/simulation_draft.rs")).unwrap();
    let options =
        fs::read_to_string(root.join("driving_adapters/ui/components/options_contract.rs"))
            .unwrap();
    let chart_action =
        fs::read_to_string(root.join("driving_adapters/ui/components/chart_simulation_action.rs"))
            .unwrap();

    assert!(application.contains("AssetSimulationPort"));
    assert!(!application.contains("leptos"));
    assert!(!application.to_lowercase().contains("echarts"));
    assert!(!port.contains("leptos"));
    assert!(mock.contains("Long Call Spread"));
    assert!(mock.contains("heatmap_fixture"));
    assert!(mock.contains("PayoffPointSnapshot"));
    assert!(mock.contains("MetricSentiment"));
    assert!(composition.contains("MockAssetSimulationAdapter"));
    assert!(!composition.contains("asset_simulation::AssetSimulationSnapshot"));
    assert!(page.contains("asset_simulation_use_case"));
    assert!(!page.contains("MockAssetSimulationAdapter"));
    assert!(page.contains("SimulationPayoffChart"));
    assert!(payoff.contains("render_chart"));
    assert!(payoff.contains("current_pnl"));
    assert!(payoff.contains("expiration_pnl"));
    assert!(!payoff.contains("MockAssetSimulationAdapter"));
    assert!(heatmap.contains("<table"));
    assert!(!heatmap.to_lowercase().contains("echarts"));
    assert!(scenario.contains("type=\"range\""));
    assert!(scenario.contains("ScenarioSelection"));
    assert!(position.contains("Edit Position"));
    assert!(position.contains("/options") && position.contains("/chart"));
    assert!(draft.contains("localStorage"));
    assert!(draft.contains("optima.simulation-draft.v1"));
    assert!(options.contains("upsert_draft_leg"));
    assert!(chart_action.contains("upsert_draft_leg"));
    assert!(!draft.contains("AssetSimulationPort"));
}

#[test]
fn options_chain_is_html_and_options_plotly_has_explicit_lifecycle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let chain =
        fs::read_to_string(root.join("driving_adapters/ui/components/options_chain.rs")).unwrap();
    let plot =
        fs::read_to_string(root.join("driving_adapters/ui/plotly/asset_options.rs")).unwrap();
    assert!(chain.contains("<table"));
    assert!(!chain.contains("Plotly"));
    assert!(plot.contains("Plotly.react"));
    assert!(plot.contains("purge_plot(HOST_ID)"));
    assert!(plot.contains("displayModeBar:false"));
    assert!(!plot.to_lowercase().contains("echarts"));
    assert!(!plot.contains("MockAssetOptionsAdapter"));
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
