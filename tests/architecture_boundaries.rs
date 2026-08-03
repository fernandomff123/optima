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
