//! Offline catalog of evidenced Cboe option product specifications.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortResult,
    domain::options::OptionContractSpecification,
    driven_ports::for_resolving_option_contract_specifications::{
        ForResolvingOptionContractSpecifications, OptionContractIdentity,
    },
};

/// Source reviewed 2026-08-20: Cboe, "S&P 500 Index Options Product
/// Specifications", contract multiplier $100 for SPX and SPXW:
/// https://www.cboe.com/tradable_products/sp_500/spx_options/specifications/
#[derive(Debug, Default, Clone, Copy)]
pub struct CboeOptionContractSpecificationsAdapter;

#[async_trait]
impl ForResolvingOptionContractSpecifications for CboeOptionContractSpecificationsAdapter {
    async fn resolve_option_contract_specification(
        &self,
        contract: OptionContractIdentity<'_>,
    ) -> PortResult<Option<OptionContractSpecification>> {
        let Some(catalog_reviewed_at) = NaiveDate::from_ymd_opt(2026, 8, 20) else {
            return Err(crate::hexagon::PortError::Unavailable(
                "invalid embedded catalog review date".to_string(),
            ));
        };
        let root = contract.root.trim().to_ascii_uppercase();
        let specification = match root.as_str() {
            "SPX" | "SPXW" => OptionContractSpecification::new(
                root,
                100.0,
                "USD",
                "cboe_spx_options_product_specifications",
                Some(catalog_reviewed_at),
                None,
            ),
            _ => None,
        };
        Ok(specification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolves_only_evidenced_exact_roots() {
        let adapter = CboeOptionContractSpecificationsAdapter;
        for root in ["SPX", "SPXW"] {
            let specification = adapter
                .resolve_option_contract_specification(OptionContractIdentity {
                    root,
                    occ_symbol: "contract",
                })
                .await
                .unwrap()
                .unwrap();
            assert_eq!(specification.contract_multiplier, 100.0);
            assert_eq!(specification.currency, "USD");
            assert_eq!(specification.effective_from, None);
            assert_eq!(
                specification.catalog_reviewed_at,
                NaiveDate::from_ymd_opt(2026, 8, 20)
            );
        }
        for root in ["SPY", "XSP", "UNKNOWN", "SPX1", "SPXW1"] {
            assert!(
                adapter
                    .resolve_option_contract_specification(OptionContractIdentity {
                        root,
                        occ_symbol: "adjusted-or-unknown",
                    })
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }
}
