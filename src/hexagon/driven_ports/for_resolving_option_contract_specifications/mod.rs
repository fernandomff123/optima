//! Conversation required to resolve evidenced option product specifications.

use async_trait::async_trait;

use crate::hexagon::{PortResult, domain::options::OptionContractSpecification};

#[derive(Debug, Clone, Copy)]
pub struct OptionContractIdentity<'a> {
    pub root: &'a str,
    pub occ_symbol: &'a str,
}

#[async_trait]
pub trait ForResolvingOptionContractSpecifications: Send + Sync {
    async fn resolve_option_contract_specification(
        &self,
        contract: OptionContractIdentity<'_>,
    ) -> PortResult<Option<OptionContractSpecification>>;
}
