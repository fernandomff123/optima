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
async fn duckdb_initialization_storage_failures_order_limit_and_recovery_are_idempotent() {
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
    assert_eq!(
        adapter
            .recover_interrupted_data_refresh_runs(now + Duration::seconds(1))
            .await
            .expect("recover"),
        1
    );
    assert_eq!(
        adapter
            .recover_interrupted_data_refresh_runs(now + Duration::seconds(2))
            .await
            .expect("idempotent recover"),
        0
    );
    let recent = adapter
        .load_recent_data_refresh_runs(1)
        .await
        .expect("load");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, "running");
    assert_eq!(recent[0].state, DataRefreshState::Failed);
    assert_eq!(recent[0].failures.len(), 1);
    std::fs::remove_file(path).expect("remove temporary database");
}
