use crate::{
    domain::asset::{AssetCapability, AssetSymbol},
    ports::asset_options::{AssetOptionsSnapshot, ContractDetailSnapshot, SmilePointSnapshot},
};

use super::chain_fixture::chain;

pub(super) fn aapl_snapshot(symbol: AssetSymbol) -> AssetOptionsSnapshot {
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
        chain: chain(),
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
