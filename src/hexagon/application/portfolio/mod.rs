//! Portfolio use cases.

use std::collections::BTreeMap;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::hexagon::{
    PortError, PortResult,
    domain::portfolio::{
        CashMovement, CurrencyExchange, Portfolio, PortfolioEvent, Position, Trade,
    },
    driven_ports::{
        for_loading_portfolios::ForLoadingPortfolios, for_storing_portfolios::ForStoringPortfolios,
    },
    driving_ports::for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
};

pub struct PortfolioApplication<PortfolioLoader, PortfolioStore> {
    portfolio_loader: PortfolioLoader,
    portfolio_store: PortfolioStore,
}

impl<PortfolioLoader, PortfolioStore> PortfolioApplication<PortfolioLoader, PortfolioStore> {
    pub fn new(portfolio_loader: PortfolioLoader, portfolio_store: PortfolioStore) -> Self {
        Self {
            portfolio_loader,
            portfolio_store,
        }
    }
}

impl<PortfolioLoader, PortfolioStore> PortfolioApplication<PortfolioLoader, PortfolioStore>
where
    PortfolioLoader: ForLoadingPortfolios,
    PortfolioStore: ForStoringPortfolios,
{
    async fn load(&self, id: &str) -> PortResult<Portfolio> {
        self.portfolio_loader
            .load_portfolio(id)
            .await?
            .ok_or_else(|| PortError::NotFound(format!("portfolio '{id}' was not found")))
    }

    async fn record(&self, id: &str, event: PortfolioEvent) -> PortResult<()> {
        let mut portfolio = self.load(id).await?;
        portfolio
            .record(event)
            .map_err(|error| PortError::InvalidRequest(error.to_string()))?;
        self.portfolio_store.store_portfolio(&portfolio).await
    }
}

#[async_trait]
impl<PortfolioLoader, PortfolioStore> ForManagingPortfolios
    for PortfolioApplication<PortfolioLoader, PortfolioStore>
where
    PortfolioLoader: ForLoadingPortfolios,
    PortfolioStore: ForStoringPortfolios,
{
    async fn create_portfolio(&self, command: CreatePortfolio) -> PortResult<()> {
        if self
            .portfolio_loader
            .load_portfolio(&command.id)
            .await?
            .is_some()
        {
            return Err(PortError::Conflict(format!(
                "portfolio '{}' already exists",
                command.id
            )));
        }
        let portfolio = Portfolio::new(command.id, command.name, command.base_currency)
            .map_err(|error| PortError::InvalidRequest(error.to_string()))?;
        self.portfolio_store.store_portfolio(&portfolio).await
    }

    async fn portfolio(&self, portfolio_id: &str) -> PortResult<Portfolio> {
        self.load(portfolio_id).await
    }

    async fn record_cash_movement(
        &self,
        portfolio_id: &str,
        movement: CashMovement,
    ) -> PortResult<()> {
        self.record(portfolio_id, PortfolioEvent::CashMovement(movement))
            .await
    }

    async fn record_option_trade(&self, portfolio_id: &str, trade: Trade) -> PortResult<()> {
        self.record(portfolio_id, PortfolioEvent::Trade(trade))
            .await
    }

    async fn record_currency_exchange(
        &self,
        portfolio_id: &str,
        exchange: CurrencyExchange,
    ) -> PortResult<()> {
        self.record(portfolio_id, PortfolioEvent::CurrencyExchange(exchange))
            .await
    }

    async fn check_balance(&self, portfolio_id: &str) -> PortResult<BTreeMap<String, Decimal>> {
        Ok(self.load(portfolio_id).await?.cash_balances())
    }

    async fn list_positions(&self, portfolio_id: &str) -> PortResult<Vec<Position>> {
        Ok(self.load(portfolio_id).await?.positions())
    }

    async fn list_transactions(&self, portfolio_id: &str) -> PortResult<Vec<PortfolioEvent>> {
        Ok(self.load(portfolio_id).await?.events().to_vec())
    }
}
