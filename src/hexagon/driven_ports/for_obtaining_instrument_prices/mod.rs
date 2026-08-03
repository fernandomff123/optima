//! Conversation required to obtain prices for portfolio instruments.

use async_trait::async_trait;

use crate::hexagon::{
    PortResult,
    domain::{portfolio::Instrument, portfolio_valuation::InstrumentPrice},
};

#[async_trait]
pub trait ForObtainingInstrumentPrices: Send + Sync {
    /// Returns one price slot for each input instrument, preserving its order.
    async fn obtain_instrument_prices(
        &self,
        instruments: &[Instrument],
    ) -> PortResult<Vec<Option<InstrumentPrice>>>;
}
