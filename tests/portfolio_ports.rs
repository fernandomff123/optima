use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use polars_options::hexagon::{
    PortResult,
    application::portfolio::PortfolioApplication,
    domain::portfolio::{CashMovement, CashMovementKind, Currency, Money, Portfolio, decimal},
    driven_ports::{
        for_loading_portfolios::ForLoadingPortfolios, for_storing_portfolios::ForStoringPortfolios,
    },
    driving_ports::for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
};

type Portfolios = Arc<Mutex<HashMap<String, Portfolio>>>;

struct PortfolioLoaderMock(Portfolios);
struct PortfolioStoreMock(Portfolios);

#[async_trait]
impl ForLoadingPortfolios for PortfolioLoaderMock {
    async fn load_portfolio(&self, id: &str) -> PortResult<Option<Portfolio>> {
        Ok(self
            .0
            .lock()
            .expect("test mutex must be usable")
            .get(id)
            .cloned())
    }
}

#[async_trait]
impl ForStoringPortfolios for PortfolioStoreMock {
    async fn store_portfolio(&self, portfolio: &Portfolio) -> PortResult<()> {
        self.0
            .lock()
            .expect("test mutex must be usable")
            .insert(portfolio.id.clone(), portfolio.clone());
        Ok(())
    }
}

fn app() -> PortfolioApplication<PortfolioLoaderMock, PortfolioStoreMock> {
    let portfolios = Arc::new(Mutex::new(HashMap::new()));
    PortfolioApplication::new(
        PortfolioLoaderMock(Arc::clone(&portfolios)),
        PortfolioStoreMock(portfolios),
    )
}

#[tokio::test]
async fn creates_a_portfolio_and_checks_its_balance_through_the_driving_port() {
    let app = app();
    app.create_portfolio(CreatePortfolio {
        id: "main".into(),
        name: "Principal".into(),
        base_currency: Currency::eur(),
    })
    .await
    .unwrap();

    app.record_cash_movement(
        "main",
        CashMovement::new(
            "deposit-1",
            Utc.with_ymd_and_hms(2026, 8, 3, 10, 0, 0).unwrap(),
            CashMovementKind::Deposit,
            Money::new(decimal("1250").unwrap(), Currency::eur()),
        )
        .unwrap(),
    )
    .await
    .unwrap();

    let balances = app.check_balance("main").await.unwrap();
    assert_eq!(balances["EUR"], decimal("1250").unwrap());
    assert_eq!(app.list_transactions("main").await.unwrap().len(), 1);
    let portfolio = app.portfolio("main").await.unwrap();
    assert_eq!(portfolio.name, "Principal");
    assert_eq!(portfolio.events().len(), 1);
}

#[tokio::test]
async fn refuses_to_create_the_same_portfolio_twice() {
    let app = app();
    let command = CreatePortfolio {
        id: "main".into(),
        name: "Principal".into(),
        base_currency: Currency::eur(),
    };
    app.create_portfolio(command.clone()).await.unwrap();

    let error = app.create_portfolio(command).await.unwrap_err();

    assert!(error.to_string().contains("already exists"));
}
