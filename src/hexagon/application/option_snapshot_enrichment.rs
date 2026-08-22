//! Provider-neutral enrichment of option snapshots with evidenced specifications.

use std::collections::BTreeSet;

use crate::hexagon::{
    PortError, PortResult,
    domain::options::{OptionIngestionWarning, Snapshot},
    driven_ports::for_resolving_option_contract_specifications::{
        ForResolvingOptionContractSpecifications, OptionContractIdentity,
        OptionContractSpecificationResolution,
    },
};

pub struct OptionSnapshotEnrichment<ContractSpecifications> {
    contract_specifications: ContractSpecifications,
}

impl<ContractSpecifications> OptionSnapshotEnrichment<ContractSpecifications> {
    pub fn new(contract_specifications: ContractSpecifications) -> Self {
        Self {
            contract_specifications,
        }
    }

    #[cfg(test)]
    pub(crate) fn contract_specifications(&self) -> &ContractSpecifications {
        &self.contract_specifications
    }
}

impl<ContractSpecifications> OptionSnapshotEnrichment<ContractSpecifications>
where
    ContractSpecifications: ForResolvingOptionContractSpecifications,
{
    pub async fn enrich(&self, snapshot: &mut Snapshot) -> PortResult<()> {
        let identities: Vec<_> = snapshot
            .chains
            .iter()
            .flat_map(|chain| {
                chain
                    .contratos
                    .iter()
                    .map(|contract| OptionContractIdentity {
                        root: chain.root.clone(),
                        occ_symbol: contract.occ_symbol.clone(),
                    })
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let resolutions = self
            .contract_specifications
            .resolve_option_contract_specifications(&identities)
            .await?;
        let requested: BTreeSet<_> = identities.into_iter().collect();
        let resolved: BTreeSet<_> = resolutions.keys().cloned().collect();
        if resolved != requested {
            return Err(incompatible_identity_set());
        }

        let mut unresolved_roots = BTreeSet::new();
        for chain in &mut snapshot.chains {
            for contract in &mut chain.contratos {
                let identity = OptionContractIdentity {
                    root: chain.root.clone(),
                    occ_symbol: contract.occ_symbol.clone(),
                };
                match resolutions.get(&identity) {
                    Some(OptionContractSpecificationResolution::Found(specification)) => {
                        contract.contract_specification = Some(specification.clone());
                    }
                    Some(OptionContractSpecificationResolution::NotFound) => {
                        unresolved_roots.insert(chain.root.clone());
                        contract.contract_specification = None;
                    }
                    None => return Err(incompatible_identity_set()),
                }
            }
        }
        let currency = single_evidenced_currency(snapshot.chains.iter().flat_map(|chain| {
            chain.contratos.iter().map(|contract| {
                contract
                    .contract_specification
                    .as_ref()
                    .map(|specification| specification.currency.as_str())
            })
        }));
        snapshot.contratos = snapshot
            .chains
            .iter()
            .flat_map(|chain| chain.contratos.iter().cloned())
            .collect();
        if unresolved_roots.is_empty()
            && let (Some(underlying), Some(currency)) = (&mut snapshot.underlying_price, currency)
        {
            underlying.currency = Some(currency);
        }
        for warning in unresolved_roots
            .into_iter()
            .map(|root| OptionIngestionWarning::ContractSpecificationUnavailable { root })
        {
            snapshot.ingestion_diagnostics.record_warning(warning);
        }
        Ok(())
    }
}

pub(crate) fn single_evidenced_currency<'a>(
    specifications: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<String> {
    let mut currencies = BTreeSet::new();
    for currency in specifications {
        currencies.insert(currency?);
    }
    if currencies.len() == 1 {
        currencies.into_iter().next().map(str::to_owned)
    } else {
        None
    }
}

fn incompatible_identity_set() -> PortError {
    PortError::Unavailable(
        "contract specification resolver returned an incompatible identity set".to_string(),
    )
}
