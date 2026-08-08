//! Coordinates offline migration of volatility term structures.
use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_volatility_term_structures::ForCountingVolatilityTermStructures,
        for_loading_volatility_term_structure_archive::ForLoadingVolatilityTermStructureArchive,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
    },
    driving_ports::for_migrating_volatility_term_structures::{
        ForMigratingVolatilityTermStructures, VolatilityTermStructureMigrationReport,
    },
};

pub struct VolatilityTermStructureMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}
impl<Source, Target> VolatilityTermStructureMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}

#[async_trait::async_trait]
impl<Source, Target> ForMigratingVolatilityTermStructures
    for VolatilityTermStructureMigrationApplication<Source, Target>
where
    Source: ForLoadingVolatilityTermStructureArchive,
    Target: ForStoringVolatilityTermStructures + ForCountingVolatilityTermStructures,
{
    async fn migrate_volatility_term_structures(
        &self,
    ) -> PortResult<VolatilityTermStructureMigrationReport> {
        let structures = self.source.load_volatility_term_structure_archive().await?;
        let source_points = structures.iter().map(|item| item.points.len() as u64).sum();
        for structure in &structures {
            self.target.store_term_structure(structure).await?;
        }
        Ok(VolatilityTermStructureMigrationReport {
            structures: structures.len() as u64,
            source_points,
            target_points: self.target.count_volatility_term_structure_points().await?,
        })
    }
}
