use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{NaiveDate, TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::{
        duckdb::option_chains::DuckDbOptionChainsAdapter,
        sqlite::{option_data::SqliteOptionDataAdapter, option_snapshots},
    },
    hexagon::{
        domain::options::{ContratoOpcao, OptionChain, OptionType, Snapshot},
        driven_ports::{
            for_loading_option_chains::ForLoadingOptionChains,
            for_storing_option_chains::ForStoringOptionChains,
        },
    },
};
use sqlx::sqlite::SqlitePoolOptions;

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn sample_snapshot() -> Snapshot {
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).expect("valid expiration");
    let contracts = vec![
        contract("SPY   260918C00500000", OptionType::Call, expiration, 500.0),
        contract("SPY   260918P00490000", OptionType::Put, expiration, 490.0),
    ];
    Snapshot {
        ticker: "SPY".to_string(),
        timestamp_utc: Utc
            .with_ymd_and_hms(2026, 8, 7, 20, 15, 0)
            .single()
            .expect("valid snapshot time"),
        contratos: contracts.clone(),
        chains: vec![OptionChain {
            root: "SPY".to_string(),
            contratos: contracts,
        }],
    }
}

fn contract(
    occ_symbol: &str,
    option_type: OptionType,
    expiration: NaiveDate,
    strike: f64,
) -> ContratoOpcao {
    ContratoOpcao {
        occ_symbol: occ_symbol.to_string(),
        option_type,
        strike,
        expiration,
        bid: 10.0,
        ask: 10.4,
        mid: 10.2,
        spread: 0.4,
        volume: 123.0,
        open_interest: 4_567.0,
        delta: 0.5,
        gamma: 0.02,
        vega: 0.15,
        theta: -0.05,
        rho: 0.03,
        theo: 10.25,
        implied_volatility: Some(0.22),
    }
}

async fn assert_option_chain_contract(
    adapter: &(impl ForLoadingOptionChains + ForStoringOptionChains),
) {
    let snapshot = sample_snapshot();
    let market_close = Utc
        .with_ymd_and_hms(2026, 8, 7, 20, 0, 0)
        .single()
        .expect("valid market close");

    assert_eq!(
        adapter
            .store_option_chain(&snapshot, market_close)
            .await
            .expect("first snapshot must be stored"),
        1
    );
    assert_eq!(
        adapter
            .store_option_chain(&snapshot, market_close)
            .await
            .expect("duplicate storage must be idempotent"),
        0
    );
    assert_eq!(
        adapter
            .load_option_chain(" spy ")
            .await
            .expect("stored snapshot must load"),
        Some(snapshot)
    );
    assert_eq!(
        adapter
            .load_option_chain("MISSING")
            .await
            .expect("missing ticker is not a storage error"),
        None
    );
}

#[tokio::test]
async fn sqlite_satisfies_option_chain_storage_contract() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite must open");
    option_snapshots::initialize(&pool)
        .await
        .expect("SQLite option schema must initialize");
    let adapter = SqliteOptionDataAdapter::new(pool);

    assert_option_chain_contract(&adapter).await;
}

#[tokio::test]
async fn duckdb_satisfies_option_chain_storage_contract_with_columnar_rows() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-contract-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    adapter
        .initialize()
        .await
        .expect("DuckDB option schema must initialize");

    assert_option_chain_contract(&adapter).await;
    assert_eq!(
        adapter.counts().await.expect("DuckDB counts must load"),
        (1, 2)
    );

    std::fs::remove_file(path).expect("temporary DuckDB file must be removable");
}

#[tokio::test]
async fn duckdb_removes_the_obsolete_hash_without_losing_snapshot_metadata() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-hash-migration-{}-{sequence}.duckdb",
        std::process::id()
    ));
    {
        let connection = duckdb::Connection::open(&path).expect("legacy DuckDB must open");
        connection
            .execute_batch(
                "CREATE TABLE option_snapshots (
                    snapshot_id VARCHAR PRIMARY KEY,
                    ticker VARCHAR NOT NULL,
                    observed_at TIMESTAMPTZ NOT NULL,
                    market_close TIMESTAMPTZ,
                    payload_hash VARCHAR NOT NULL UNIQUE,
                    format_version INTEGER NOT NULL,
                    UNIQUE (ticker, market_close)
                );
                INSERT INTO option_snapshots VALUES
                    ('old-id', 'SPX', TIMESTAMPTZ '2026-08-07 20:15:00+00',
                     TIMESTAMPTZ '2026-08-07 20:00:00+00', 'obsolete', 1);",
            )
            .expect("legacy schema must be created");
    }

    DuckDbOptionChainsAdapter::new(&path)
        .initialize()
        .await
        .expect("legacy hash migration must succeed");

    let connection = duckdb::Connection::open(&path).expect("migrated DuckDB must open");
    let hash_columns: u64 = connection
        .query_row(
            "SELECT COUNT(*) FROM information_schema.columns
             WHERE table_name = 'option_snapshots' AND column_name = 'payload_hash'",
            [],
            |row| row.get(0),
        )
        .expect("schema must be inspectable");
    let snapshots: u64 = connection
        .query_row("SELECT COUNT(*) FROM option_snapshots", [], |row| {
            row.get(0)
        })
        .expect("snapshot count must be readable");

    assert_eq!(hash_columns, 0);
    assert_eq!(snapshots, 1);
    drop(connection);
    std::fs::remove_file(path).expect("temporary DuckDB file must be removable");
}
