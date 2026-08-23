//! Intraday-simulation preparation use case.

use async_trait::async_trait;

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        simulation::{IntradaySimulationMarket, SimulationCatalog},
        volatility_surface::VolatilitySurface,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_option_chains::ForObtainingOptionChains,
    },
    driving_ports::{
        for_preparing_intraday_simulations::ForPreparingIntradaySimulations,
        for_viewing_intraday_options::{ForViewingIntradayOptions, IntradayOptionsMarket},
    },
};

pub struct IntradaySimulationApplication<OptionChains, LivePrices, TradingCalendar> {
    option_chains: OptionChains,
    live_prices: LivePrices,
    trading_calendar: TradingCalendar,
}

impl<OptionChains, LivePrices, TradingCalendar>
    IntradaySimulationApplication<OptionChains, LivePrices, TradingCalendar>
{
    pub fn new(
        option_chains: OptionChains,
        live_prices: LivePrices,
        trading_calendar: TradingCalendar,
    ) -> Self {
        Self {
            option_chains,
            live_prices,
            trading_calendar,
        }
    }
}

#[async_trait]
impl<OptionChains, LivePrices, TradingCalendar> ForPreparingIntradaySimulations
    for IntradaySimulationApplication<OptionChains, LivePrices, TradingCalendar>
where
    OptionChains: ForObtainingOptionChains,
    LivePrices: ForObtainingLivePrices,
    TradingCalendar: ForConsultingTradingCalendar,
{
    async fn intraday_market(&self, ticker: &str) -> PortResult<IntradaySimulationMarket> {
        if !self
            .trading_calendar
            .is_regular_session(chrono::Utc::now())?
        {
            return Err(PortError::Conflict(
                "intraday simulation requires a regular trading session".to_string(),
            ));
        }
        let ticker = normalized_ticker(ticker)?;
        let snapshot = self.option_chains.obtain_option_chain(&ticker).await?;
        let price = self.live_prices.obtain_live_price(&ticker).await?;
        if !price.price.is_finite() || price.price <= 0.0 {
            return Err(PortError::Unavailable(
                "current reference price is unavailable".to_string(),
            ));
        }
        Ok(IntradaySimulationMarket {
            snapshot,
            spot: price.price,
        })
    }
}

#[async_trait]
impl<OptionChains, LivePrices, TradingCalendar> ForViewingIntradayOptions
    for IntradaySimulationApplication<OptionChains, LivePrices, TradingCalendar>
where
    OptionChains: ForObtainingOptionChains,
    LivePrices: ForObtainingLivePrices,
    TradingCalendar: ForConsultingTradingCalendar,
{
    async fn intraday_options(&self, ticker: &str) -> PortResult<IntradayOptionsMarket> {
        let market = self.intraday_market(ticker).await?;
        let catalog = SimulationCatalog::from_snapshot(
            normalized_ticker(ticker)?,
            &market.snapshot,
            market.spot,
        );
        let volatility_surface = VolatilitySurface::from_snapshot(&market.snapshot, market.spot);
        Ok(IntradayOptionsMarket {
            market,
            catalog,
            volatility_surface,
        })
    }
}

fn normalized_ticker(ticker: &str) -> PortResult<String> {
    let ticker = ticker.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || !ticker
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err(PortError::InvalidRequest("invalid ticker".to_string()));
    }
    Ok(ticker)
}
