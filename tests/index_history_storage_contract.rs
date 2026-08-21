use std::sync::atomic::{AtomicU64, Ordering};

use chrono::NaiveDate;
use hexagonal_backend::{
    driven_adapters::duckdb::index_history::DuckDbIndexHistoryAdapter,
    hexagon::{
        domain::index_history::{DailyIndexPrice, IndexHistory},
        driven_ports::{
            for_loading_index_history::ForLoadingIndexHistory,
            for_storing_index_history::ForStoringIndexHistory,
        },
    },
};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn history() -> IndexHistory {
    IndexHistory {
        ticker: "VIX".to_string(),
        daily_prices: vec![DailyIndexPrice {
            date: NaiveDate::from_ymd_opt(2026, 8, 7).expect("valid date"),
            open: Some(16.0),
            high: Some(17.0),
            low: Some(15.5),
            close: 16.5,
        }],
    }
}

async fn assert_contract(adapter: &(impl ForLoadingIndexHistory + ForStoringIndexHistory)) {
    let expected = history();
    assert_eq!(
        adapter
            .store_index_history(&expected)
            .await
            .expect("first history must store"),
        1
    );
    assert_eq!(
        adapter
            .store_index_history(&expected)
            .await
            .expect("duplicate must be idempotent"),
        0
    );
    assert_eq!(
        adapter
            .load_index_history(" vix ")
            .await
            .expect("history must load"),
        expected
    );
}

#[tokio::test]
async fn duckdb_satisfies_index_history_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-index-history-contract-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbIndexHistoryAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
