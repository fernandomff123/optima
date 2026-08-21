use hexagonal_backend::{
    driven_adapters::duckdb::saved_strategies::DuckDbSavedStrategiesAdapter,
    hexagon::{
        domain::saved_strategy::{SavedStrategyLeg, StrategySide},
        driven_ports::{
            for_loading_strategies::ForLoadingStrategies,
            for_storing_strategies::ForStoringStrategies,
        },
    },
};
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn assert_contract(adapter: &(impl ForLoadingStrategies + ForStoringStrategies)) {
    let legs = vec![SavedStrategyLeg {
        occ_symbol: "SPY260101P00090000".into(),
        side: StrategySide::Buy,
        quantity: 2,
        entry_price: 2.5,
    }];
    let stored = adapter
        .store_strategy("Put hedge", "SPY", &legs)
        .await
        .expect("strategy must store");
    assert_eq!(stored.legs, legs);
    assert_eq!(
        adapter.load_strategies().await.expect("must load"),
        vec![stored.clone()]
    );

    assert!(
        adapter
            .delete_strategy(stored.id)
            .await
            .expect("must delete")
    );
    assert!(
        !adapter
            .delete_strategy(stored.id)
            .await
            .expect("must be idempotent")
    );
    assert!(
        adapter
            .load_strategies()
            .await
            .expect("must load")
            .is_empty()
    );
}

#[tokio::test]
async fn duckdb_satisfies_saved_strategy_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-saved-strategies-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbSavedStrategiesAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
