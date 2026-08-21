use std::{collections::BTreeSet, fs, path::Path};

use hexagonal_backend::driving_adapters::http::{
    CANONICAL_ALIASES, EXISTING_CANONICAL_ROUTES, NON_CANONICAL_SYNCHRONIZATION_ALIASES,
};

#[test]
fn canonical_routes_are_prefixed_and_path_method_pairs_do_not_collide() {
    let mut routes = BTreeSet::new();
    for &(canonical, _, method, _) in CANONICAL_ALIASES {
        assert!(
            canonical.starts_with("/api/"),
            "non-canonical prefix: {canonical}"
        );
        assert!(
            routes.insert((method, canonical)),
            "route collision: {method} {canonical}"
        );
    }
    for &(method, canonical) in EXISTING_CANONICAL_ROUTES {
        assert!(
            canonical.starts_with("/api/"),
            "non-canonical prefix: {canonical}"
        );
        assert!(
            routes.insert((method, canonical)),
            "route collision: {method} {canonical}"
        );
    }
}

#[test]
fn aliases_and_canonical_routes_are_registered_with_the_same_handler() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/driving_adapters/http/mod.rs"))
        .expect("HTTP adapter source must be readable");
    for &(canonical, alias, _, handler) in CANONICAL_ALIASES {
        let canonical_registration = format!("\"{canonical}\"");
        let alias_registration = format!("\"{alias}\"");
        assert!(
            source.contains(&canonical_registration),
            "missing canonical route {canonical}"
        );
        assert!(
            source.contains(&alias_registration),
            "missing alias {alias}"
        );
        assert!(
            source.matches(handler).count() >= 3,
            "{canonical} and {alias} must share {handler}"
        );
    }
}

#[test]
fn granular_synchronization_has_no_public_api_equivalent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/driving_adapters/http/mod.rs"))
        .expect("HTTP adapter source must be readable");
    for &(_, alias) in NON_CANONICAL_SYNCHRONIZATION_ALIASES {
        assert!(
            source.contains(alias),
            "operational alias disappeared: {alias}"
        );
        assert!(
            !source.contains(&format!("\"/api{alias}\"")),
            "granular synchronization became public: /api{alias}"
        );
    }
    assert!(source.contains("\"/api/data-refresh/status\""));
    assert!(source.contains("\"/api/data-refresh\""));
}

#[test]
fn canonical_http_boundary_uses_api_models_and_keeps_composition_out() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/driving_adapters/http/mod.rs"))
        .expect("HTTP adapter source must be readable");
    assert!(!source.contains("Json<crate::hexagon::domain"));
    assert!(!source.contains("configurator::configure"));
    assert!(!source.contains("driven_adapters::"));
    assert!(source.contains("Json<api_models::"));

    let application = root.join("src/hexagon/application");
    for entry in fs::read_dir(application).expect("application directory must exist") {
        let path = entry.expect("application entry must be readable").path();
        if path.is_dir() {
            let module = fs::read_to_string(path.join("mod.rs"))
                .expect("application module must be readable");
            assert!(!module.contains("api_models"));
            assert!(!module.to_ascii_lowercase().contains("axum"));
        }
    }
}

#[test]
fn api_models_is_a_wire_only_independent_crate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("crates/api_models/Cargo.toml"))
        .expect("api_models manifest must be readable");
    for forbidden in ["hexagonal_backend", "axum", "tokio", "duckdb", "reqwest"] {
        assert!(
            !manifest.contains(forbidden),
            "api_models depends on {forbidden}"
        );
    }
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("dependencies section must exist")
        .split("[dev-dependencies]")
        .next()
        .expect("dependencies section must end");
    for dependency in dependencies.lines().filter(|line| line.contains(" = ")) {
        let name = dependency.split_whitespace().next().unwrap_or_default();
        assert!(
            ["chrono", "serde", "rust_decimal"].contains(&name),
            "unexpected api_models dependency: {name}"
        );
    }
    let source = fs::read_to_string(root.join("crates/api_models/src/lib.rs"))
        .expect("api_models source must be readable");
    for forbidden in [
        "hexagon",
        "application",
        "driving_adapters",
        "driven_adapters",
        "axum",
        "tokio",
    ] {
        assert!(
            !source.contains(forbidden),
            "api_models mentions {forbidden}"
        );
    }
    for business_marker in [
        "impl MarketHistory",
        "impl StrategySimulationResult",
        "fn calculate",
        "fn validate",
    ] {
        assert!(
            !source.contains(business_marker),
            "wire crate contains behavior: {business_marker}"
        );
    }
}

#[test]
fn route_inventory_documentation_matches_the_registered_canonical_routes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(root.join("docs/public-api.md"))
        .expect("public API inventory must be readable");
    for &(canonical, alias, method, _) in CANONICAL_ALIASES {
        assert!(
            docs.contains(&format!("| {method} | `{alias}`")),
            "alias missing from inventory: {method} {alias}"
        );
        assert!(
            docs.contains(&format!("`{canonical}`")),
            "canonical route missing from inventory: {method} {canonical}"
        );
    }
}

#[test]
fn websocket_path_and_single_composition_remain_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let legacy = fs::read_to_string(root.join("src/driving_adapters/http/legacy_server.rs"))
        .expect("legacy HTTP source must be readable");
    assert!(legacy.contains(".route(\"/api/assets/live\", get(asset_live_prices))"));
    let server = fs::read_to_string(root.join("src/bin/web_server.rs"))
        .expect("server source must be readable");
    assert_eq!(server.matches("configurator::configure()").count(), 1);
}
