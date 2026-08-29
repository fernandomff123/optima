use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_options::{
        AssetOptionsFailure, AssetOptionsPort, AssetOptionsSnapshot, ContractDetailSnapshot,
        OptionChainRowSnapshot, OptionSideSnapshot, OptionsScenario, SmilePointSnapshot,
    },
};

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

fn side(values: [&'static str; 7]) -> OptionSideSnapshot {
    OptionSideSnapshot {
        last: values[0],
        bid: values[1],
        ask: values[2],
        iv: values[3],
        delta: values[4],
        open_interest: values[5],
        volume: values[6],
    }
}

fn row(
    strike: &'static str,
    call: [&'static str; 7],
    put: [&'static str; 7],
    is_atm: bool,
    is_selected: bool,
) -> OptionChainRowSnapshot {
    OptionChainRowSnapshot {
        strike,
        is_atm,
        is_selected,
        call: side(call),
        put: side(put),
    }
}

fn aapl_snapshot(symbol: AssetSymbol) -> AssetOptionsSnapshot {
    AssetOptionsSnapshot {
        symbol,
        name: "Apple Inc.",
        venue: "NASDAQ",
        price: "191.13",
        spot: 191.13,
        absolute_change: "+2.34",
        percentage_change: "+1.24%",
        change_positive: true,
        capabilities: vec![
            AssetCapability::Overview,
            AssetCapability::Chart,
            AssetCapability::Options,
            AssetCapability::Volatility,
            AssetCapability::Gex,
            AssetCapability::Simulation,
        ],
        expiration: "17 May 2025",
        dte: "36",
        strike_range: "±10 ATM",
        iv_rank: "42.3",
        put_call_oi: "0.79",
        earnings: "May 1, 2025 (13 DTE)",
        chain: vec![
            row(
                "180.00",
                ["12.85", "12.60", "13.10", "24.1", "0.83", "8,432", "3,287"],
                ["0.33", "0.32", "0.35", "24.7", "-0.17", "12,356", "2,914"],
                false,
                false,
            ),
            row(
                "182.50",
                ["9.80", "9.60", "10.00", "23.7", "0.78", "9,215", "3,671"],
                ["0.46", "0.45", "0.48", "24.1", "-0.22", "13,284", "3,126"],
                false,
                false,
            ),
            row(
                "185.00",
                ["7.30", "7.15", "7.45", "23.4", "0.72", "10,134", "4,025"],
                ["0.64", "0.63", "0.66", "23.6", "-0.28", "14,902", "3,580"],
                false,
                false,
            ),
            row(
                "187.50",
                ["5.35", "5.20", "5.50", "23.0", "0.65", "11,278", "4,832"],
                ["0.88", "0.87", "0.91", "23.1", "-0.35", "16,278", "4,213"],
                false,
                false,
            ),
            row(
                "190.00",
                ["3.85", "3.75", "3.95", "22.6", "0.58", "12,562", "5,249"],
                ["1.23", "1.21", "1.26", "22.6", "-0.42", "17,642", "4,812"],
                false,
                false,
            ),
            row(
                "192.50",
                ["2.70", "2.61", "2.79", "22.2", "0.50", "13,845", "6,012"],
                ["1.70", "1.67", "1.74", "22.2", "-0.50", "18,954", "5,612"],
                true,
                false,
            ),
            row(
                "195.00",
                ["1.82", "1.74", "1.90", "21.9", "0.42", "14,932", "6,401"],
                ["2.32", "2.28", "2.36", "21.9", "-0.58", "19,876", "6,203"],
                false,
                false,
            ),
            row(
                "197.50",
                ["1.15", "1.08", "1.22", "21.6", "0.33", "16,274", "7,318"],
                ["3.10", "3.05", "3.15", "21.6", "-0.67", "20,485", "6,157"],
                false,
                true,
            ),
            row(
                "200.00",
                ["0.68", "0.63", "0.72", "21.4", "0.24", "17,112", "6,582"],
                ["4.05", "4.00", "4.12", "21.4", "-0.76", "21,177", "5,241"],
                false,
                false,
            ),
            row(
                "202.50",
                ["0.38", "0.34", "0.41", "21.2", "0.16", "18,065", "5,404"],
                ["5.20", "5.12", "5.28", "21.2", "-0.84", "21,568", "4,312"],
                false,
                false,
            ),
            row(
                "205.00",
                ["0.20", "0.17", "0.22", "21.1", "0.10", "18,943", "3,762"],
                ["6.55", "6.45", "6.66", "21.1", "-0.90", "21,943", "3,284"],
                false,
                false,
            ),
        ],
        smile: vec![
            SmilePointSnapshot {
                strike: 142.5,
                call_iv: 27.5,
                put_iv: 25.5,
            },
            SmilePointSnapshot {
                strike: 150.0,
                call_iv: 26.5,
                put_iv: 24.9,
            },
            SmilePointSnapshot {
                strike: 157.5,
                call_iv: 25.6,
                put_iv: 24.0,
            },
            SmilePointSnapshot {
                strike: 165.0,
                call_iv: 24.2,
                put_iv: 22.6,
            },
            SmilePointSnapshot {
                strike: 172.5,
                call_iv: 22.8,
                put_iv: 21.2,
            },
            SmilePointSnapshot {
                strike: 180.0,
                call_iv: 21.2,
                put_iv: 19.8,
            },
            SmilePointSnapshot {
                strike: 187.5,
                call_iv: 20.1,
                put_iv: 18.8,
            },
            SmilePointSnapshot {
                strike: 195.0,
                call_iv: 19.7,
                put_iv: 18.5,
            },
            SmilePointSnapshot {
                strike: 202.5,
                call_iv: 20.0,
                put_iv: 18.8,
            },
            SmilePointSnapshot {
                strike: 210.0,
                call_iv: 20.8,
                put_iv: 19.5,
            },
            SmilePointSnapshot {
                strike: 217.5,
                call_iv: 21.6,
                put_iv: 20.2,
            },
            SmilePointSnapshot {
                strike: 225.0,
                call_iv: 22.5,
                put_iv: 21.0,
            },
            SmilePointSnapshot {
                strike: 232.5,
                call_iv: 23.5,
                put_iv: 22.0,
            },
            SmilePointSnapshot {
                strike: 240.0,
                call_iv: 24.6,
                put_iv: 23.0,
            },
            SmilePointSnapshot {
                strike: 245.0,
                call_iv: 25.7,
                put_iv: 24.0,
            },
        ],
        contract: ContractDetailSnapshot {
            title: "AAPL 17 MAY 2025 195 CALL",
            price: "1.15",
            change: "+0.14  +13.86%",
            bid: "1.08",
            ask: "1.22",
            bid_size: "152",
            ask_size: "163",
            metrics: vec![
                ("Mid", "1.15"),
                ("Last Size", "25"),
                ("Volume", "7,318"),
                ("Open Interest", "16,274"),
                ("IV", "21.6%"),
                ("IV Rank", "42.3"),
                ("Delta", "0.33"),
                ("Gamma", "0.0128"),
                ("Vega", "0.206"),
                ("Theta", "-0.028"),
                ("Rho", "0.016"),
            ],
            facts: vec![
                ("Expiration", "17 May 2025"),
                ("DTE", "36"),
                ("Strike", "195.00"),
                ("Type", "Call"),
                ("Multiplier", "100"),
                ("Trading Class", "AAPL"),
                ("Exchange", "NASDAQ"),
                ("Currency", "USD"),
            ],
        },
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
