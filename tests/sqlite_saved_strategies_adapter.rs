use hexagonal_backend::{
    driven_adapters::sqlite::saved_strategies::SqliteSavedStrategiesAdapter,
    hexagon::{
        application::saved_strategies::SavedStrategiesApplication,
        domain::saved_strategy::{SavedStrategyLeg, StrategySide},
        driving_ports::for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
    },
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn application_manages_saved_strategies_through_sqlite_ports() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let adapter = SqliteSavedStrategiesAdapter::new(pool);
    let application = SavedStrategiesApplication::new(adapter.clone(), adapter);

    let stored = application
        .save_strategy(SaveStrategy {
            name: "Put hedge".into(),
            ticker: "SPY".into(),
            legs: vec![SavedStrategyLeg {
                occ_symbol: "SPY260101P00090000".into(),
                side: StrategySide::Buy,
                quantity: 1,
                entry_price: 2.5,
            }],
        })
        .await
        .unwrap();

    assert_eq!(application.list_strategies().await.unwrap(), vec![stored]);
}
