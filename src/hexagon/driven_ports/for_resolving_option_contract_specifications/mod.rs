//! Conversation required to resolve evidenced option product specifications.

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::options::OptionContractSpecification};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OptionContractIdentity {
    pub root: String,
    pub occ_symbol: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionContractSpecificationResolution {
    Found(OptionContractSpecification),
    NotFound,
}

#[async_trait]
pub trait ForResolvingOptionContractSpecifications: Send + Sync {
    async fn resolve_option_contract_specifications(
        &self,
        contracts: &[OptionContractIdentity],
    ) -> PortResult<BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>>;
}
