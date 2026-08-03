use polars_options::driven_adapters::sqlite::option_snapshots;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn test_pipeline_msgpack_e_sqlite() {
    let response: polars_options::driven_adapters::cboe::CboeResponse =
        serde_json::from_str(include_str!("fixtures/snapshot.json"))
            .expect("o fixture deve conter um DTO CBOE válido");
    let expected = polars_options::driven_adapters::cboe::response_to_snapshot("SPY", response)
        .expect("o DTO deve ser convertido para o domínio");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    option_snapshots::initialize(&pool).await.unwrap();

    option_snapshots::save_snapshot(&pool, &expected, expected.timestamp_utc)
        .await
        .unwrap();
    let loaded = option_snapshots::load_latest(&pool, "SPY")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(loaded, expected);
}
