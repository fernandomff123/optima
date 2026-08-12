use chrono::{Duration, TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::duckdb::data_refresh_runs::DuckDbDataRefreshRunsAdapter,
    hexagon::{
        domain::data_refresh::{
            DataRefreshFailure, DataRefreshOrigin, DataRefreshRun, DataRefreshState,
        },
        driven_ports::{
            for_loading_data_refresh_runs::ForLoadingDataRefreshRuns,
            for_storing_data_refresh_runs::ForStoringDataRefreshRuns,
        },
    },
};

#[tokio::test]
async fn duckdb_initialization_storage_failures_order_limit_and_running_load_are_idempotent() {
    let path = std::env::temp_dir().join(format!("refresh-runs-{}.duckdb", std::process::id()));
    let adapter = DuckDbDataRefreshRunsAdapter::new(&path);
    adapter.initialize().await.expect("initialize");
    adapter.initialize().await.expect("idempotent");
    let now = Utc
        .with_ymd_and_hms(2026, 8, 13, 22, 0, 0)
        .single()
        .expect("fixture");
    let mut old = DataRefreshRun::running(
        "old".into(),
        DataRefreshOrigin::Scheduled,
        now - Duration::minutes(1),
        now.date_naive(),
    );
    old.finish(
        now,
        1,
        1,
        vec![DataRefreshFailure {
            ticker: "XLK".into(),
            operation: "history".into(),
            error: "controlled".into(),
        }],
        Some(now + Duration::minutes(5)),
    )
    .expect("finish");
    adapter.store_data_refresh_run(&old).await.expect("store");
    let running = DataRefreshRun::running(
        "running".into(),
        DataRefreshOrigin::Manual,
        now,
        now.date_naive(),
    );
    adapter
        .store_data_refresh_run(&running)
        .await
        .expect("store running");
    let running_runs = adapter
        .load_running_data_refresh_runs()
        .await
        .expect("load running");
    assert_eq!(running_runs, vec![running.clone()]);
    let recent = adapter
        .load_recent_data_refresh_runs(1)
        .await
        .expect("load");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, "running");
    assert_eq!(recent[0].state, DataRefreshState::Running);
    std::fs::remove_file(path).expect("remove temporary database");
}

#[test]
fn duckdb_adapter_only_loads_and_stores_refresh_runs() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/driven_adapters/duckdb/data_refresh_runs.rs"),
    )
    .expect("adapter source");
    assert!(!source.contains(".interrupt("));
    assert!(!source.contains("recover_interrupted"));
    assert!(!source.contains("next_attempt_at ="));
}
