//! Saved-strategy management use cases.

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::saved_strategy::SavedStrategy,
    driven_ports::{
        for_loading_strategies::ForLoadingStrategies, for_storing_strategies::ForStoringStrategies,
    },
    driving_ports::for_managing_saved_strategies::{ForManagingSavedStrategies, SaveStrategy},
};

pub struct SavedStrategiesApplication<Loader, Store> {
    loader: Loader,
    store: Store,
}

impl<Loader, Store> SavedStrategiesApplication<Loader, Store> {
    pub fn new(loader: Loader, store: Store) -> Self {
        Self { loader, store }
    }
}

#[async_trait]
impl<Loader, Store> ForManagingSavedStrategies for SavedStrategiesApplication<Loader, Store>
where
    Loader: ForLoadingStrategies,
    Store: ForStoringStrategies,
{
    async fn list_strategies(&self) -> PortResult<Vec<SavedStrategy>> {
        self.loader.load_strategies().await
    }

    async fn save_strategy(&self, command: SaveStrategy) -> PortResult<SavedStrategy> {
        let name = command.name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(PortError::InvalidRequest(
                "strategy name must contain between 1 and 80 characters".into(),
            ));
        }
        let ticker = command.ticker.trim().to_ascii_uppercase();
        if ticker.is_empty()
            || !ticker
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '^')
        {
            return Err(PortError::InvalidRequest("invalid strategy ticker".into()));
        }
        if command.legs.is_empty()
            || command.legs.iter().any(|leg| {
                leg.occ_symbol.trim().is_empty()
                    || leg.quantity == 0
                    || !leg.entry_price.is_finite()
                    || leg.entry_price < 0.0
            })
        {
            return Err(PortError::InvalidRequest("invalid strategy legs".into()));
        }
        self.store
            .store_strategy(name, &ticker, &command.legs)
            .await
    }

    async fn delete_strategy(&self, id: i64) -> PortResult<()> {
        if id <= 0 {
            return Err(PortError::InvalidRequest(
                "strategy id must be positive".into(),
            ));
        }
        if !self.store.delete_strategy(id).await? {
            return Err(PortError::NotFound(format!(
                "saved strategy '{id}' was not found"
            )));
        }
        Ok(())
    }
}
