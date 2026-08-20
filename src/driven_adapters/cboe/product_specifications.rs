//! Offline catalog of evidenced Cboe option product specifications.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortResult,
    domain::options::OptionContractSpecification,
    driven_ports::for_resolving_option_contract_specifications::{
        ForResolvingOptionContractSpecifications, OptionContractIdentity,
        OptionContractSpecificationResolution,
    },
};

/// Source reviewed 2026-08-20: Cboe, "S&P 500 Index Options Product
/// Specifications", contract multiplier $100 for SPX and SPXW:
/// https://www.cboe.com/tradable_products/sp_500/spx_options/specifications/
#[derive(Debug, Default, Clone, Copy)]
pub struct CboeOptionContractSpecificationsAdapter;

#[async_trait]
impl ForResolvingOptionContractSpecifications for CboeOptionContractSpecificationsAdapter {
    async fn resolve_option_contract_specifications(
        &self,
        contracts: &[OptionContractIdentity],
    ) -> PortResult<BTreeMap<OptionContractIdentity, OptionContractSpecificationResolution>> {
        let Some(catalog_reviewed_at) = NaiveDate::from_ymd_opt(2026, 8, 20) else {
            return Err(crate::hexagon::PortError::Unavailable(
                "invalid embedded catalog review date".to_string(),
            ));
        };
        Ok(contracts
            .iter()
            .cloned()
            .map(|identity| {
                let root = identity.root.trim().to_ascii_uppercase();
                let resolution = match root.as_str() {
                    "SPX" | "SPXW" => OptionContractSpecification::new(
                        root,
                        100.0,
                        "USD",
                        "cboe_spx_options_product_specifications",
                        Some(catalog_reviewed_at),
                        None,
                    )
                    .map_or(
                        OptionContractSpecificationResolution::NotFound,
                        OptionContractSpecificationResolution::Found,
                    ),
                    _ => OptionContractSpecificationResolution::NotFound,
                };
                (identity, resolution)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolves_only_evidenced_exact_roots() {
        let adapter = CboeOptionContractSpecificationsAdapter;
        let identities: Vec<_> = ["SPX", "SPXW", "SPY", "XSP", "UNKNOWN", "SPX1", "SPXW1"]
            .into_iter()
            .map(|root| OptionContractIdentity {
                root: root.to_string(),
                occ_symbol: format!("{root}-contract"),
            })
            .collect();
        let resolutions = adapter
            .resolve_option_contract_specifications(&identities)
            .await
            .unwrap();

        for identity in &identities[..2] {
            let OptionContractSpecificationResolution::Found(specification) =
                &resolutions[identity]
            else {
                panic!("standard root must be resolved");
            };
            assert_eq!(specification.contract_multiplier, 100.0);
            assert_eq!(specification.currency, "USD");
            assert_eq!(specification.effective_from, None);
            assert_eq!(
                specification.catalog_reviewed_at,
                NaiveDate::from_ymd_opt(2026, 8, 20)
            );
        }
        for identity in &identities[2..] {
            assert_eq!(
                resolutions[identity],
                OptionContractSpecificationResolution::NotFound
            );
        }
    }
}
