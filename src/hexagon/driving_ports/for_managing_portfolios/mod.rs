//! Conversation offered to actors managing portfolios.

use std::collections::BTreeMap;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::hexagon::{
    PortResult,
    domain::portfolio::{
        CashMovement, Currency, CurrencyExchange, Portfolio, PortfolioEvent, Position, Trade,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePortfolio {
    pub id: String,
    pub name: String,
    pub base_currency: Currency,
}

/// Provided interface containing portfolio commands and queries.
#[async_trait]
pub trait ForManagingPortfolios: Send + Sync {
    async fn create_portfolio(&self, command: CreatePortfolio) -> PortResult<()>;

    async fn portfolio(&self, portfolio_id: &str) -> PortResult<Portfolio>;

    async fn record_cash_movement(
        &self,
        portfolio_id: &str,
        movement: CashMovement,
    ) -> PortResult<()>;

    async fn record_option_trade(&self, portfolio_id: &str, trade: Trade) -> PortResult<()>;

    async fn record_currency_exchange(
        &self,
        portfolio_id: &str,
        exchange: CurrencyExchange,
    ) -> PortResult<()>;

    async fn check_balance(&self, portfolio_id: &str) -> PortResult<BTreeMap<String, Decimal>>;

    async fn list_positions(&self, portfolio_id: &str) -> PortResult<Vec<Position>>;

    async fn list_transactions(&self, portfolio_id: &str) -> PortResult<Vec<PortfolioEvent>>;
}
