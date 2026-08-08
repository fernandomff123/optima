//! Portfolio-position valuation use case.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        options::{OccSymbol, Snapshot},
        portfolio::Instrument,
        portfolio_valuation::{InstrumentPrice, ValuedPosition},
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_portfolios::ForLoadingPortfolios,
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_option_chains::ForObtainingOptionChains,
    },
    driving_ports::for_viewing_portfolio_positions::ForViewingPortfolioPositions,
};

pub struct PricingCollaborators<Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions> {
    calendar: Calendar,
    live_prices: LivePrices,
    market_history: MarketHistory,
    live_options: LiveOptions,
    stored_options: StoredOptions,
}

impl<Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions>
    PricingCollaborators<Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions>
{
    pub fn new(
        calendar: Calendar,
        live_prices: LivePrices,
        market_history: MarketHistory,
        live_options: LiveOptions,
        stored_options: StoredOptions,
    ) -> Self {
        Self {
            calendar,
            live_prices,
            market_history,
            live_options,
            stored_options,
        }
    }
}

pub struct PortfolioValuationApplication<PortfolioLoader, Pricing> {
    portfolio_loader: PortfolioLoader,
    pricing: Pricing,
}

impl<PortfolioLoader, Pricing> PortfolioValuationApplication<PortfolioLoader, Pricing> {
    pub fn new(portfolio_loader: PortfolioLoader, pricing: Pricing) -> Self {
        Self {
            portfolio_loader,
            pricing,
        }
    }
}

#[async_trait]
impl<PortfolioLoader, Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions>
    ForViewingPortfolioPositions
    for PortfolioValuationApplication<
        PortfolioLoader,
        PricingCollaborators<Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions>,
    >
where
    PortfolioLoader: ForLoadingPortfolios,
    Calendar: ForConsultingTradingCalendar,
    LivePrices: ForObtainingLivePrices,
    MarketHistory: ForLoadingMarketHistory,
    LiveOptions: ForObtainingOptionChains,
    StoredOptions: ForLoadingOptionChains,
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
        let regular_session = self.pricing.calendar.is_regular_session(Utc::now())?;
        let mut snapshots = BTreeMap::new();
        let mut valued = Vec::with_capacity(positions.len());
        for position in positions {
            // A missing quote makes only that position unvalued; it does not hide the portfolio.
            let market_price = self
                .price(&position.instrument, regular_session, &mut snapshots)
                .await
                .unwrap_or(None);
            let market_value = market_price.as_ref().and_then(|price| {
                Some(
                    price.price
                        * position.quantity.to_f64()?
                        * position.instrument.contract_multiplier().to_f64()?,
                )
            });
            valued.push(ValuedPosition {
                instrument: position.instrument,
                quantity: position.quantity,
                market_price,
                market_value,
            });
        }
        Ok(valued)
    }
}

impl<PortfolioLoader, Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions>
    PortfolioValuationApplication<
        PortfolioLoader,
        PricingCollaborators<Calendar, LivePrices, MarketHistory, LiveOptions, StoredOptions>,
    >
where
    Calendar: ForConsultingTradingCalendar,
    LivePrices: ForObtainingLivePrices,
    MarketHistory: ForLoadingMarketHistory,
    LiveOptions: ForObtainingOptionChains,
    StoredOptions: ForLoadingOptionChains,
{
    async fn price(
        &self,
        instrument: &Instrument,
        regular_session: bool,
        snapshots: &mut BTreeMap<String, Snapshot>,
    ) -> PortResult<Option<InstrumentPrice>> {
        match instrument {
            Instrument::Equity { ticker } if regular_session => {
                let quote = self.pricing.live_prices.obtain_live_price(ticker).await?;
                Ok(Some(InstrumentPrice {
                    price: quote.price,
                    currency: quote.currency,
                    source: "live market price".to_string(),
                    observed_at: chrono::DateTime::from_timestamp(quote.market_time, 0)
                        .unwrap_or_else(Utc::now),
                }))
            }
            Instrument::Equity { ticker } => {
                let history = self
                    .pricing
                    .market_history
                    .load_market_history(ticker)
                    .await?;
                Ok(history.daily_quotes.last().and_then(|quote| {
                    Some(InstrumentPrice {
                        price: quote.close?,
                        currency: history
                            .currency
                            .clone()
                            .unwrap_or_else(|| "USD".to_string()),
                        source: "end-of-day market price".to_string(),
                        observed_at: quote.timestamp,
                    })
                }))
            }
            Instrument::Option { occ_symbol } => {
                let occ = OccSymbol::parse(occ_symbol)
                    .map_err(|error| PortError::InvalidRequest(error.to_string()))?;
                let ticker = if occ.root.eq_ignore_ascii_case("SPXW") {
                    "SPX".to_string()
                } else {
                    occ.root
                };
                if !snapshots.contains_key(&ticker) {
                    let snapshot = if regular_session {
                        Some(
                            self.pricing
                                .live_options
                                .obtain_option_chain(&ticker)
                                .await?,
                        )
                    } else {
                        self.pricing
                            .stored_options
                            .load_option_chain(&ticker)
                            .await?
                    };
                    let Some(snapshot) = snapshot else {
                        return Ok(None);
                    };
                    snapshots.insert(ticker.clone(), snapshot);
                }
                let snapshot = snapshots.get(&ticker).ok_or_else(|| {
                    PortError::Unavailable("option snapshot cache is inconsistent".to_string())
                })?;
                Ok(snapshot
                    .contratos
                    .iter()
                    .find(|contract| contract.occ_symbol == *occ_symbol)
                    .map(|contract| InstrumentPrice {
                        price: contract.mid,
                        currency: "USD".to_string(),
                        source: if regular_session {
                            "live option chain"
                        } else {
                            "stored option snapshot"
                        }
                        .to_string(),
                        observed_at: snapshot.timestamp_utc,
                    }))
            }
        }
    }
}
