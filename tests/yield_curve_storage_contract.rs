use std::sync::atomic::{AtomicU64, Ordering};

use chrono::NaiveDate;
use hexagonal_backend::{
    driven_adapters::{
        duckdb::yield_curves::DuckDbYieldCurvesAdapter,
        sqlite::{yield_curves, yield_curves_port::SqliteYieldCurvesAdapter},
    },
    hexagon::{
        domain::treasury::YieldCurve,
        driven_ports::{
            for_loading_yield_curves::ForLoadingYieldCurves,
            for_storing_yield_curves::ForStoringYieldCurves,
        },
    },
};
use sqlx::sqlite::SqlitePoolOptions;

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn curve() -> YieldCurve {
    YieldCurve {
        date: NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
        m1: Some(0.043),
        m2: Some(0.042),
        m3: Some(0.041),
        m6: Some(0.04),
        y1: Some(0.039),
        y2: Some(0.038),
        y3: Some(0.039),
        y5: Some(0.04),
        y7: Some(0.041),
        y10: Some(0.042),
        y20: Some(0.044),
        y30: Some(0.045),
    }
}

async fn assert_contract(adapter: &(impl ForLoadingYieldCurves + ForStoringYieldCurves)) {
    let expected = curve();
    adapter
        .store_yield_curves(std::slice::from_ref(&expected))
        .await
        .expect("curve must store");
    assert_eq!(
        adapter
            .load_yield_curve(expected.date)
            .await
            .expect("curve must load"),
        Some(expected)
    );
}

#[tokio::test]
async fn sqlite_satisfies_yield_curve_contract() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("SQLite must open");
    yield_curves::initialize(&pool)
        .await
        .expect("SQLite schema must initialize");
    assert_contract(&SqliteYieldCurvesAdapter::new(pool)).await;
}

#[tokio::test]
async fn duckdb_satisfies_yield_curve_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-yield-curve-contract-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbYieldCurvesAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
