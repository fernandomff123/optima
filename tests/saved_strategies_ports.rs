use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use hexagonal_backend::hexagon::{
    PortResult,
    application::saved_strategies::SavedStrategiesApplication,
    domain::saved_strategy::{SavedStrategy, SavedStrategyLeg, StrategySide},
    driven_ports::{
        for_loading_strategies::ForLoadingStrategies, for_storing_strategies::ForStoringStrategies,
    },
    driving_ports::for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
};

#[derive(Clone, Default)]
struct StrategiesMock(Arc<Mutex<Vec<SavedStrategy>>>);

#[async_trait]
impl ForLoadingStrategies for StrategiesMock {
    async fn load_strategies(&self) -> PortResult<Vec<SavedStrategy>> {
        Ok(self.0.lock().expect("test mutex must be usable").clone())
    }
}

#[async_trait]
impl ForStoringStrategies for StrategiesMock {
    async fn store_strategy(
        &self,
        name: &str,
        ticker: &str,
        legs: &[SavedStrategyLeg],
    ) -> PortResult<SavedStrategy> {
        let strategy = SavedStrategy {
            id: 1,
            name: name.to_string(),
            ticker: ticker.to_string(),
            legs: legs.to_vec(),
            updated_at: Utc::now(),
        };
        self.0
            .lock()
            .expect("test mutex must be usable")
            .push(strategy.clone());
        Ok(strategy)
    }

    async fn delete_strategy(&self, id: i64) -> PortResult<bool> {
        let mut strategies = self.0.lock().expect("test mutex must be usable");
        let previous = strategies.len();
        strategies.retain(|strategy| strategy.id != id);
        Ok(previous != strategies.len())
    }
}

#[tokio::test]
async fn manages_strategies_through_driving_and_mocked_driven_ports() {
    let adapter = StrategiesMock::default();
    let application = SavedStrategiesApplication::new(adapter.clone(), adapter);

    let stored = application
        .save_strategy(SaveStrategy {
            name: "Long call".to_string(),
            ticker: " spy ".to_string(),
            legs: vec![SavedStrategyLeg {
                occ_symbol: "SPY260101C00100000".to_string(),
                side: StrategySide::Buy,
                quantity: 1,
                entry_price: 5.0,
            }],
        })
        .await
        .expect("valid strategy must be stored");

    assert_eq!(stored.ticker, "SPY");
    assert_eq!(application.list_strategies().await.unwrap().len(), 1);
    application.delete_strategy(stored.id).await.unwrap();
    assert!(application.list_strategies().await.unwrap().is_empty());
}

#[tokio::test]
async fn rejects_invalid_strategy_before_storing_it() {
    let adapter = StrategiesMock::default();
    let application = SavedStrategiesApplication::new(adapter.clone(), adapter);

    let error = application
        .save_strategy(SaveStrategy {
            name: " ".to_string(),
            ticker: "SPY".to_string(),
            legs: Vec::new(),
        })
        .await
        .unwrap_err();

    assert!(error.to_string().contains("name"));
}
