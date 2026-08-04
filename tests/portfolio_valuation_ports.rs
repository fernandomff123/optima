use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use hexagonal_backend::hexagon::{
    PortResult,
    application::portfolio_valuation::PortfolioValuationApplication,
    domain::{
        portfolio::{
            Currency, Instrument, Money, Portfolio, PortfolioEvent, Trade, TradeSide, decimal,
        },
        portfolio_valuation::InstrumentPrice,
    },
    driven_ports::{
        for_loading_portfolios::ForLoadingPortfolios,
        for_obtaining_instrument_prices::ForObtainingInstrumentPrices,
    },
    driving_ports::for_viewing_portfolio_positions::ForViewingPortfolioPositions,
};

struct PortfolioLoaderMock(Portfolio);

#[async_trait]
impl ForLoadingPortfolios for PortfolioLoaderMock {
    async fn load_portfolio(&self, _id: &str) -> PortResult<Option<Portfolio>> {
        Ok(Some(self.0.clone()))
    }
}

struct InstrumentPricesMock;

#[async_trait]
impl ForObtainingInstrumentPrices for InstrumentPricesMock {
    async fn obtain_instrument_prices(
        &self,
        instruments: &[Instrument],
    ) -> PortResult<Vec<Option<InstrumentPrice>>> {
        Ok(instruments
            .iter()
            .map(|_| {
                Some(InstrumentPrice {
                    price: 2.5,
                    currency: "USD".to_string(),
                    source: "test actor".to_string(),
                    observed_at: Utc.with_ymd_and_hms(2026, 8, 3, 15, 0, 0).unwrap(),
                })
            })
            .collect())
    }
}

#[tokio::test]
async fn values_option_positions_through_a_mocked_price_actor() {
    let mut portfolio = Portfolio::new("main", "Principal", Currency::eur()).unwrap();
    let trade = Trade::new(
        "trade-1",
        Instrument::Option {
            occ_symbol: "SPXW  260821C05000000".to_string(),
        },
        TradeSide::Buy,
        Utc.with_ymd_and_hms(2026, 8, 3, 14, 0, 0).unwrap(),
        decimal("2").unwrap(),
        Money::new(decimal("2").unwrap(), Currency::new("USD").unwrap()),
    )
    .unwrap();
    portfolio.record(PortfolioEvent::Trade(trade)).unwrap();
    let application =
        PortfolioValuationApplication::new(PortfolioLoaderMock(portfolio), InstrumentPricesMock);

    let positions = application.valued_positions("main").await.unwrap();

    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].market_value, Some(500.0));
}
