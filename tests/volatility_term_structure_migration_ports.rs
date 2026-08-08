use hexagonal_backend::hexagon::{
    PortResult,
    application::volatility_term_structure_migration::VolatilityTermStructureMigrationApplication,
    domain::volatility::TermStructure,
    driven_ports::{
        for_counting_volatility_term_structures::ForCountingVolatilityTermStructures,
        for_loading_volatility_term_structure_archive::ForLoadingVolatilityTermStructureArchive,
        for_storing_volatility_term_structures::ForStoringVolatilityTermStructures,
    },
    driving_ports::for_migrating_volatility_term_structures::ForMigratingVolatilityTermStructures,
};

struct SourceMock;
struct TargetMock;

#[async_trait::async_trait]
impl ForLoadingVolatilityTermStructureArchive for SourceMock {
    async fn load_volatility_term_structure_archive(&self) -> PortResult<Vec<TermStructure>> {
        Ok(Vec::new())
    }
}

#[async_trait::async_trait]
impl ForStoringVolatilityTermStructures for TargetMock {
    async fn store_term_structure(&self, _term_structure: &TermStructure) -> PortResult<u64> {
        Ok(0)
    }
}

#[async_trait::async_trait]
impl ForCountingVolatilityTermStructures for TargetMock {
    async fn count_volatility_term_structure_points(&self) -> PortResult<u64> {
        Ok(1_184)
    }
}

#[tokio::test]
async fn application_coordinates_volatility_migration_through_ports() {
    let report = VolatilityTermStructureMigrationApplication::new(SourceMock, TargetMock)
        .migrate_volatility_term_structures()
        .await
        .expect("migration must succeed");
    assert_eq!(report.structures, 0);
    assert_eq!(report.target_points, 1_184);
}
