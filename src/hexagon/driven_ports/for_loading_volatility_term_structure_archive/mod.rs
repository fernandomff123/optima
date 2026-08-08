//! Conversation required to export volatility term structures during migration.
use crate::hexagon::{PortResult, domain::volatility::TermStructure};
use async_trait::async_trait;

#[async_trait]
pub trait ForLoadingVolatilityTermStructureArchive: Send + Sync {
    async fn load_volatility_term_structure_archive(&self) -> PortResult<Vec<TermStructure>>;
}
