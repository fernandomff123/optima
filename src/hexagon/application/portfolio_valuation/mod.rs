//! Portfolio-position valuation use case.

use async_trait::async_trait;
use rust_decimal::prelude::ToPrimitive;

use crate::hexagon::{
    PortError, PortResult,
    domain::portfolio_valuation::ValuedPosition,
    driven_ports::{
        for_loading_portfolios::ForLoadingPortfolios,
        for_obtaining_instrument_prices::ForObtainingInstrumentPrices,
    },
    driving_ports::for_viewing_portfolio_positions::ForViewingPortfolioPositions,
};

pub struct PortfolioValuationApplication<PortfolioLoader, InstrumentPrices> {
    portfolio_loader: PortfolioLoader,
    instrument_prices: InstrumentPrices,
}

impl<PortfolioLoader, InstrumentPrices>
    PortfolioValuationApplication<PortfolioLoader, InstrumentPrices>
{
    pub fn new(portfolio_loader: PortfolioLoader, instrument_prices: InstrumentPrices) -> Self {
        Self {
            portfolio_loader,
            instrument_prices,
        }
    }
}

#[async_trait]
impl<PortfolioLoader, InstrumentPrices> ForViewingPortfolioPositions
    for PortfolioValuationApplication<PortfolioLoader, InstrumentPrices>
where
    PortfolioLoader: ForLoadingPortfolios,
    InstrumentPrices: ForObtainingInstrumentPrices,
{
    async fn valued_positions(&self, portfolio_id: &str) -> PortResult<Vec<ValuedPosition>> {
        let portfolio = self
            .portfolio_loader
            .load_portfolio(portfolio_id)
            .await?
            .ok_or_else(|| {
                PortError::NotFound(format!("portfolio '{portfolio_id}' was not found"))
            })?;
        let positions = portfolio.positions();
        let instruments = positions
            .iter()
            .map(|position| position.instrument.clone())
            .collect::<Vec<_>>();
        let prices = self
            .instrument_prices
            .obtain_instrument_prices(&instruments)
            .await?;
        if prices.len() != positions.len() {
            return Err(PortError::Unavailable(
                "instrument price actor returned an invalid result count".to_string(),
            ));
        }
        Ok(positions
            .into_iter()
            .zip(prices)
            .map(|(position, market_price)| {
                let market_value = market_price.as_ref().and_then(|price| {
                    Some(
                        price.price
                            * position.quantity.to_f64()?
                            * position.instrument.contract_multiplier().to_f64()?,
                    )
                });
                ValuedPosition {
                    instrument: position.instrument,
                    quantity: position.quantity,
                    market_price,
                    market_value,
                }
            })
            .collect())
    }
}
