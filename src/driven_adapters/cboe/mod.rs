//! Driven adapter for the external option-chain actor.

mod client;
mod indices_client;
mod indices_parser;
mod parser;
pub mod product_specifications;

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::{index_history::IndexHistory, options::Snapshot},
    driven_ports::{
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_obtaining_volatility_indices::ForObtainingVolatilityIndices,
    },
};

/// Obtains delayed option-chain snapshots from Cboe.
#[derive(Debug, Default, Clone, Copy)]
pub struct CboeOptionChainsAdapter;

/// Obtains complete volatility-index histories from Cboe.
#[derive(Debug, Default, Clone, Copy)]
pub struct CboeVolatilityIndicesAdapter;

#[async_trait]
impl ForObtainingOptionChains for CboeOptionChainsAdapter {
    async fn obtain_option_chain(&self, ticker: &str) -> PortResult<Snapshot> {
        let ticker = ticker.trim().to_ascii_uppercase();
        let response = client::download_snapshot(&ticker)
            .await
            .map_err(unavailable)?;
        parser::response_to_snapshot_collected_at(&ticker, response, chrono::Utc::now())
            .map_err(unavailable)
    }
}

#[async_trait]
impl ForObtainingVolatilityIndices for CboeVolatilityIndicesAdapter {
    async fn obtain_volatility_index(&self, ticker: &str) -> PortResult<IndexHistory> {
        let response = indices_client::download_indice(ticker)
            .await
            .map_err(unavailable)?;
        indices_parser::response_to_index_history(response).map_err(unavailable)
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}

pub use client::CboeResponse;
pub use parser::response_to_snapshot;
pub use parser::response_to_snapshot_collected_at;
pub use parser::{ParseError, parse_occ_symbol};
