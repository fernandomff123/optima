//! Coordinates the temporary offline migration of yield curves.

use crate::hexagon::{
    PortResult,
    driven_ports::{
        for_counting_yield_curves::ForCountingYieldCurves,
        for_loading_yield_curve_archive::ForLoadingYieldCurveArchive,
        for_storing_yield_curves::ForStoringYieldCurves,
    },
    driving_ports::for_migrating_yield_curves::{
        ForMigratingYieldCurves, YieldCurveMigrationReport,
    },
};

pub struct YieldCurveMigrationApplication<Source, Target> {
    source: Source,
    target: Target,
}

impl<Source, Target> YieldCurveMigrationApplication<Source, Target> {
    pub fn new(source: Source, target: Target) -> Self {
        Self { source, target }
    }
}

#[async_trait::async_trait]
impl<Source, Target> ForMigratingYieldCurves for YieldCurveMigrationApplication<Source, Target>
where
    Source: ForLoadingYieldCurveArchive,
    Target: ForStoringYieldCurves + ForCountingYieldCurves,
{
    async fn migrate_yield_curves(&self) -> PortResult<YieldCurveMigrationReport> {
        let curves = self.source.load_yield_curve_archive().await?;
        self.target.store_yield_curves(&curves).await?;
        Ok(YieldCurveMigrationReport {
            source_rows: curves.len() as u64,
            target_rows: self.target.count_yield_curves().await?,
        })
    }
}
