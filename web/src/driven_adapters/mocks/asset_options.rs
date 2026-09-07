use crate::{
    domain::asset::AssetSymbol,
    ports::asset_options::{
        AssetOptionsFailure, AssetOptionsPort, AssetOptionsSnapshot, OptionsScenario,
    },
};

mod chain_fixture;
mod details;
mod fixture;

use fixture::aapl_snapshot;

#[derive(Clone, Copy, Debug, Default)]
pub struct MockAssetOptionsAdapter;

impl AssetOptionsPort for MockAssetOptionsAdapter {
    fn load(
        &self,
        symbol: &AssetSymbol,
        scenario: OptionsScenario,
    ) -> Result<Option<AssetOptionsSnapshot>, AssetOptionsFailure> {
        match scenario {
            OptionsScenario::Unavailable => return Ok(None),
            OptionsScenario::RecoverableError => return Err(AssetOptionsFailure::Recoverable),
            OptionsScenario::Normal | OptionsScenario::Loading => {}
        }
        if symbol.as_str() != "AAPL" {
            return Ok(None);
        }
        Ok(Some(aapl_snapshot(symbol.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aapl_fixture_is_deterministic_and_explicitly_illustrative() {
        let symbol = AssetSymbol::new("AAPL");
        let snapshot = MockAssetOptionsAdapter
            .load(&symbol, OptionsScenario::Normal)
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.price, "191.13");
        assert_eq!(snapshot.chain.len(), 11);
        assert_eq!(
            snapshot.chain.iter().filter(|row| row.is_selected).count(),
            1
        );
        assert_eq!(snapshot.chain.iter().filter(|row| row.is_atm).count(), 1);
        assert_eq!(snapshot.smile.len(), 15);
    }

    #[test]
    fn non_aapl_assets_are_unavailable_in_this_mock() {
        assert!(
            MockAssetOptionsAdapter
                .load(&AssetSymbol::new("SPX"), OptionsScenario::Normal)
                .unwrap()
                .is_none()
        );
    }
}
