//! Coordinates offline migration of portfolio aggregates.
use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_portfolios::{ForCountingPortfolios, PortfolioCounts},
        for_loading_portfolio_archive::ForLoadingPortfolioArchive,
        for_storing_portfolios::ForStoringPortfolios,
    },
    driving_ports::for_migrating_portfolios::{ForMigratingPortfolios, PortfolioMigrationReport},
};
pub struct PortfolioMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}
impl<Source, Target> PortfolioMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}
#[async_trait::async_trait]
impl<Source, Target> ForMigratingPortfolios for PortfolioMigrationApplication<Source, Target>
where
    Source: ForLoadingPortfolioArchive,
    Target: ForStoringPortfolios + ForCountingPortfolios,
{
    async fn migrate_portfolios(&self) -> PortResult<PortfolioMigrationReport> {
        let portfolios = self.source.load_portfolio_archive().await?;
        let source = PortfolioCounts {
            portfolios: portfolios.len() as u64,
            events: portfolios
                .iter()
                .map(|portfolio| portfolio.events().len() as u64)
                .sum(),
        };
        for portfolio in &portfolios {
            self.target.store_portfolio(portfolio).await?;
        }
        Ok(PortfolioMigrationReport {
            source,
            target: self.target.count_portfolios().await?,
        })
    }
}
