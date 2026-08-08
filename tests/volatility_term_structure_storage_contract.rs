use chrono::{NaiveDate, TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::{
        duckdb::volatility_term_structures::DuckDbVolatilityTermStructuresAdapter,
        sqlite::{option_data::SqliteOptionDataAdapter, volatility_term_structures},
    },
    hexagon::{
        domain::volatility::{TermStructure, TermStructurePoint, TermStructureSource},
        driven_ports::{
            for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
            for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
        },
    },
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn structure() -> TermStructure {
    TermStructure {
        ticker: "SPX".to_string(),
        snapshot_timestamp: Utc
            .with_ymd_and_hms(2026, 8, 7, 20, 15, 0)
            .single()
            .expect("valid time"),
        treasury_date: NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
        points: vec![TermStructurePoint {
            days: 30.0,
            variance: 0.04,
            volatility: 20.0,
            source: TermStructureSource::Interpolated {
                near_expiration: NaiveDate::from_ymd_opt(2026, 8, 28).expect("valid date"),
                near_rate: 0.04,
                next_expiration: NaiveDate::from_ymd_opt(2026, 9, 4).expect("valid date"),
                next_rate: 0.041,
            },
        }],
    }
}

async fn assert_contract(
    adapter: &(impl ForLoadingVolatilityTermStructures + ForStoringVolatilityTermStructures),
) {
    let expected = structure();
    assert_eq!(
        adapter
            .store_term_structure(&expected)
            .await
            .expect("store must succeed"),
        1
    );
    assert_eq!(
        adapter
            .store_term_structure(&expected)
            .await
            .expect("duplicate must succeed"),
        0
    );
    assert_eq!(
        adapter
            .load_term_structure(" spx ")
            .await
            .expect("load must succeed"),
        Some(expected)
    );
}

#[tokio::test]
async fn sqlite_satisfies_term_structure_contract() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("SQLite must open");
    volatility_term_structures::initialize(&pool)
        .await
        .expect("schema must initialize");
    assert_contract(&SqliteOptionDataAdapter::new(pool)).await;
}

#[tokio::test]
async fn duckdb_satisfies_term_structure_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-volatility-terms-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbVolatilityTermStructuresAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
