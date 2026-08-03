use polars_options::{
    driven_adapters::sqlite::portfolio::SqlitePortfolioAdapter,
    hexagon::{
        application::portfolio::PortfolioApplication,
        domain::portfolio::Currency,
        driving_ports::for_managing_portfolios::{CreatePortfolio, ForManagingPortfolios},
    },
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn application_uses_sqlite_only_through_portfolio_ports() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let adapter = SqlitePortfolioAdapter::new(pool);
    let app = PortfolioApplication::new(adapter.clone(), adapter);

    app.create_portfolio(CreatePortfolio {
        id: "main".into(),
        name: "Principal".into(),
        base_currency: Currency::eur(),
    })
    .await
    .unwrap();

    assert!(app.check_balance("main").await.unwrap().is_empty());
}
