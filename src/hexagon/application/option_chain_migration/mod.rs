//! Coordinates an offline option-chain migration between two storage actors.

use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_option_chains::ForCountingOptionChains,
        for_loading_option_chain_archive::ForLoadingOptionChainArchive,
        for_storing_option_chains::ForStoringOptionChains,
    },
    driving_ports::for_migrating_option_chains::{
        ForMigratingOptionChains, OptionChainMigrationReport,
    },
};

pub struct OptionChainMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}

impl<Source, Target> OptionChainMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}

#[async_trait::async_trait]
impl<Source, Target> ForMigratingOptionChains for OptionChainMigrationApplication<Source, Target>
where
    Source: ForLoadingOptionChainArchive,
    Target: ForStoringOptionChains + ForCountingOptionChains,
{
    async fn migrate_option_chains(&self) -> PortResult<OptionChainMigrationReport> {
        let archived = self.source.load_option_chain_archive().await?;
        let source_contracts = archived
            .iter()
            .map(|item| item.snapshot.contratos.len() as u64)
            .sum();
        let mut inserted_snapshots = 0;
        let mut skipped_without_market_close = 0;

        for item in &archived {
            let Some(market_close) = item.market_close else {
                skipped_without_market_close += 1;
                continue;
            };
            inserted_snapshots += self
                .target
                .store_option_chain(&item.snapshot, market_close)
                .await?;
        }

        let target = self.target.count_option_chains().await?;
        Ok(OptionChainMigrationReport {
            source_snapshots: archived.len() as u64,
            source_contracts,
            inserted_snapshots,
            skipped_without_market_close,
            target_snapshots: target.snapshots,
            target_contracts: target.contracts,
        })
    }
}
