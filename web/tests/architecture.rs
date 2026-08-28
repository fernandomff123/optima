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
        assert!(
            router.contains(&format!("path!(\"{route}\")")),
            "router is missing {route}"
        );
    }
}
