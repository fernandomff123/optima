use hexagonal_backend::hexagon::{
    PortResult,
    application::portfolio_migration::PortfolioMigrationApplication,
    domain::portfolio::{Currency, Portfolio},
    driven_ports::{
        for_counting_portfolios::{ForCountingPortfolios, PortfolioCounts},
        for_loading_portfolio_archive::ForLoadingPortfolioArchive,
        for_storing_portfolios::ForStoringPortfolios,
    },
    driving_ports::for_migrating_portfolios::ForMigratingPortfolios,
};

struct SourceMock;
struct TargetMock;

#[async_trait::async_trait]
impl ForLoadingPortfolioArchive for SourceMock {
    async fn load_portfolio_archive(&self) -> PortResult<Vec<Portfolio>> {
        Ok(vec![
            Portfolio::new("main", "Principal", Currency::eur())
                .expect("portfolio fixture must be valid"),
        ])
    }
}

#[async_trait::async_trait]
impl ForStoringPortfolios for TargetMock {
    async fn store_portfolio(&self, _portfolio: &Portfolio) -> PortResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl ForCountingPortfolios for TargetMock {
    async fn count_portfolios(&self) -> PortResult<PortfolioCounts> {
        Ok(PortfolioCounts {
            portfolios: 1,
            events: 0,
        })
    }
}

#[tokio::test]
async fn application_coordinates_portfolio_migration_through_ports() {
    let report = PortfolioMigrationApplication::new(SourceMock, TargetMock)
        .migrate_portfolios()
        .await
        .expect("migration must succeed");

    assert_eq!(report.source.portfolios, 1);
    assert_eq!(report.source.events, 0);
    assert_eq!(report.target, report.source);
}
