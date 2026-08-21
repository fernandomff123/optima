use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{NaiveDate, TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::{
        cboe::CboeResponse,
        duckdb::option_chains::DuckDbOptionChainsAdapter,
        sqlite::{option_data::SqliteOptionDataAdapter, option_snapshots},
    },
    hexagon::{
        domain::options::{
            ContratoOpcao, OptionChain, OptionContractSpecification, OptionIngestionDiagnostics,
            OptionType, ProviderTimestamp, ProviderTimestampTimezone, Snapshot,
            UnderlyingPriceObservation,
        },
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
        underlying_price: UnderlyingPriceObservation::new(
            500.25,
            Some(Utc.with_ymd_and_hms(2026, 8, 7, 20, 14, 0).unwrap()),
            Some("USD".to_string()),
            "cboe_delayed_quotes",
        )
        .map(|observation| {
            observation.with_provider_timestamp(
                Some("2026-08-07T16:14:00-04:00".to_string()),
                Some(ProviderTimestampTimezone::VerifiedOffset),
            )
        }),
        collected_at: Some(Utc.with_ymd_and_hms(2026, 8, 7, 20, 15, 2).unwrap()),
        provider_timestamp: Some(ProviderTimestamp {
            raw: "2026-08-07T16:14:00-04:00".to_string(),
            timezone: ProviderTimestampTimezone::VerifiedOffset,
        }),
        ingestion_diagnostics: OptionIngestionDiagnostics {
            invalid_occ_symbol_count: 1,
            invalid_occ_symbol_samples: vec!["invalid".to_string()],
            warning_count: 0,
            warnings: Vec::new(),
        },
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
        open_interest: Some(4_567.0),
        delta: 0.5,
        gamma: Some(0.02),
        vega: 0.15,
        theta: -0.05,
        rho: 0.03,
        theo: 10.25,
        implied_volatility: Some(0.22),
        contract_specification: OptionContractSpecification::new(
            "SPY",
            50.0,
            "USD",
            "fixture_adjusted_contract",
            NaiveDate::from_ymd_opt(2026, 8, 20),
            Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        ),
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

async fn assert_nullable_market_facts_round_trip(
    adapter: &(impl ForLoadingOptionChains + ForStoringOptionChains),
) {
    let mut snapshot = sample_snapshot();
    snapshot.chains[0].contratos[0].gamma = None;
    snapshot.chains[0].contratos[0].open_interest = None;
    snapshot.chains[0].contratos[1].gamma = Some(0.0);
    snapshot.chains[0].contratos[1].open_interest = Some(0.0);
    snapshot.contratos = snapshot.chains[0].contratos.clone();
    let market_close = Utc
        .with_ymd_and_hms(2026, 8, 7, 20, 0, 0)
        .single()
        .expect("valid market close");

    assert_eq!(
        adapter
            .store_option_chain(&snapshot, market_close)
            .await
            .expect("nullable market facts must store"),
        1
    );
    let loaded = adapter
        .load_option_chain("SPY")
        .await
        .expect("nullable market facts must load")
        .expect("snapshot must exist");
    assert_eq!(loaded, snapshot);
    assert_eq!(loaded.contratos[0].gamma, None);
    assert_eq!(loaded.contratos[0].open_interest, None);
    assert_eq!(loaded.contratos[1].gamma, Some(0.0));
    assert_eq!(loaded.contratos[1].open_interest, Some(0.0));
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
async fn sqlite_preserves_null_zero_and_present_gamma_and_open_interest() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory SQLite must open");
    option_snapshots::initialize(&pool)
        .await
        .expect("SQLite option schema must initialize");
    let adapter = SqliteOptionDataAdapter::new(pool);

    assert_nullable_market_facts_round_trip(&adapter).await;
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
    adapter
        .initialize()
        .await
        .expect("DuckDB option schema migration must be repeatable");

    assert_option_chain_contract(&adapter).await;
    assert_eq!(
        adapter.counts().await.expect("DuckDB counts must load"),
        (1, 2)
    );

    std::fs::remove_file(path).expect("temporary DuckDB file must be removable");
}

#[tokio::test]
async fn duckdb_preserves_null_zero_and_present_gamma_and_open_interest() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-nullable-facts-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    adapter
        .initialize()
        .await
        .expect("DuckDB option schema must initialize");
    adapter
        .initialize()
        .await
        .expect("DuckDB option schema migration must remain idempotent");

    assert_nullable_market_facts_round_trip(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB file must be removable");
}

#[tokio::test]
async fn duckdb_persists_a_snapshot_without_spot() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-no-spot-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    let mut snapshot = sample_snapshot();
    snapshot.underlying_price = None;
    adapter
        .store_option_chain(
            &snapshot,
            Utc.with_ymd_and_hms(2026, 8, 7, 20, 0, 0).unwrap(),
        )
        .await
        .expect("snapshot without spot remains persistible");
    assert_eq!(
        adapter.load_option_chain("SPY").await.unwrap(),
        Some(snapshot)
    );
    std::fs::remove_file(path).expect("temporary DuckDB file must be removable");
}

#[tokio::test]
async fn cboe_spot_and_timestamp_survive_the_duckdb_round_trip() {
    let response: CboeResponse = serde_json::from_str(
        r#"{
            "timestamp":"2026-08-20T11:00:00-04:00",
            "data":{
                "current_price":6420.5,
                "last_trade_time":"2026-08-20T10:59:58-04:00",
                "options":[{
                    "option":"SPXW  260821C05000000","bid":1.0,"ask":1.2,
                    "volume":10.0,"open_interest":20.0,"delta":0.5,
                    "gamma":0.02,"vega":0.1,"theta":-0.03,"rho":0.01,
                    "theo":1.1,"iv":0.2
                }]
            }
        }"#,
    )
    .expect("fixture wire response must deserialize");
    let collected_at = Utc.with_ymd_and_hms(2026, 8, 20, 15, 0, 2).unwrap();
    let expected = hexagonal_backend::driven_adapters::cboe::response_to_snapshot_collected_at(
        "SPX",
        response,
        collected_at,
    )
    .expect("wire response must map");
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-wire-round-trip-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    adapter
        .store_option_chain(
            &expected,
            Utc.with_ymd_and_hms(2026, 8, 20, 20, 0, 0).unwrap(),
        )
        .await
        .expect("parsed snapshot must store");
    let loaded = adapter.load_option_chain("SPX").await.unwrap().unwrap();
    assert_eq!(loaded.underlying_price, expected.underlying_price);
    assert_eq!(loaded.provider_timestamp, expected.provider_timestamp);
    assert_eq!(
        loaded.collected_at.map(|value| value.timestamp_micros()),
        expected.collected_at.map(|value| value.timestamp_micros())
    );
    std::fs::remove_file(path).expect("temporary DuckDB file must be removable");
}

#[tokio::test]
async fn offsetless_timestamps_survive_duckdb_without_becoming_utc() {
    let response: CboeResponse = serde_json::from_str(
        r#"{
            "timestamp":"2026-08-20 11:00:00",
            "data":{
                "current_price":6420.5,
                "last_trade_time":"2026-08-20T10:59:58",
                "options":[{
                    "option":"SPXW  260821C05000000","bid":1.0,"ask":1.2,
                    "volume":10.0,"open_interest":20.0,"delta":0.5,
                    "gamma":0.02,"vega":0.1,"theta":-0.03,"rho":0.01,
                    "theo":1.1,"iv":0.2
                }]
            }
        }"#,
    )
    .unwrap();
    let collected_at = Utc.with_ymd_and_hms(2026, 8, 20, 15, 0, 2).unwrap();
    let expected = hexagonal_backend::driven_adapters::cboe::response_to_snapshot_collected_at(
        "SPX",
        response,
        collected_at,
    )
    .unwrap();
    assert_eq!(
        expected.underlying_price.as_ref().unwrap().observed_at,
        None
    );

    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-offsetless-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    adapter
        .store_option_chain(
            &expected,
            Utc.with_ymd_and_hms(2026, 8, 20, 20, 0, 0).unwrap(),
        )
        .await
        .unwrap();
    let loaded = adapter.load_option_chain("SPX").await.unwrap().unwrap();
    assert_eq!(loaded.underlying_price, expected.underlying_price);
    assert_eq!(loaded.provider_timestamp, expected.provider_timestamp);
    assert_eq!(loaded.collected_at, Some(collected_at));
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn duckdb_rolls_back_metadata_when_contract_storage_fails() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-rollback-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    let mut snapshot = sample_snapshot();
    let duplicate = snapshot.chains[0].contratos[0].clone();
    snapshot.chains[0].contratos.push(duplicate);
    assert!(
        adapter
            .store_option_chain(
                &snapshot,
                Utc.with_ymd_and_hms(2026, 8, 7, 20, 0, 0).unwrap(),
            )
            .await
            .is_err()
    );
    assert_eq!(adapter.counts().await.unwrap(), (0, 0));
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn unique_market_close_preserves_original_snapshot_and_reports_no_insert() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-option-enrichment-conflict-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbOptionChainsAdapter::new(&path);
    let market_close = Utc.with_ymd_and_hms(2026, 8, 7, 20, 0, 0).unwrap();
    let mut incomplete = sample_snapshot();
    for contract in &mut incomplete.chains[0].contratos {
        contract.contract_specification = None;
    }
    incomplete.contratos = incomplete.chains[0].contratos.clone();
    assert_eq!(
        adapter
            .store_option_chain(&incomplete, market_close)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        adapter
            .store_option_chain(&sample_snapshot(), market_close)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        adapter.load_option_chain("SPY").await.unwrap(),
        Some(incomplete)
    );
    std::fs::remove_file(path).unwrap();
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
