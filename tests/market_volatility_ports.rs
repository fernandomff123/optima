use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use hexagonal_backend::hexagon::{
    PortResult,
    application::market_volatility::MarketVolatilityApplication,
    domain::{
        index_history::{DailyIndexPrice, IndexHistory},
        volatility::{TermStructure, TermStructurePoint, TermStructureSource},
    },
    driven_ports::{
        for_loading_index_history::ForLoadingIndexHistory,
        for_loading_market_history::ForLoadingMarketHistory,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
    },
    driving_ports::for_viewing_volatility::ForViewingVolatility,
};

struct IndexHistoryMock(HashMap<String, IndexHistory>);

#[async_trait]
impl ForLoadingIndexHistory for IndexHistoryMock {
    async fn load_index_history(&self, ticker: &str) -> PortResult<IndexHistory> {
        Ok(self.0.get(ticker).cloned().unwrap_or(IndexHistory {
            ticker: ticker.to_string(),
            daily_prices: Vec::new(),
        }))
    }
}

struct OptionDataMock(TermStructure);

struct MarketHistoryMock;

#[async_trait]
impl ForLoadingMarketHistory for MarketHistoryMock {
    async fn load_market_history(
        &self,
        ticker: &str,
    ) -> PortResult<hexagonal_backend::hexagon::domain::market_history::MarketHistory> {
        Ok(
            hexagonal_backend::hexagon::domain::market_history::MarketHistory {
                ticker: ticker.to_string(),
                currency: None,
                exchange_timezone: None,
                daily_quotes: Vec::new(),
                dividends: Vec::new(),
                splits: Vec::new(),
            },
        )
    }
}

#[async_trait]
impl ForLoadingVolatilityTermStructures for OptionDataMock {
    async fn load_term_structure(&self, _ticker: &str) -> PortResult<Option<TermStructure>> {
        Ok(Some(self.0.clone()))
    }

    async fn load_term_structure_at_or_before(
        &self,
        _ticker: &str,
        _instant: DateTime<Utc>,
    ) -> PortResult<Option<TermStructure>> {
        Ok(Some(self.0.clone()))
    }

    async fn load_constant_maturity_volatility_history(
        &self,
        _ticker: &str,
        _target_days: f64,
    ) -> PortResult<
        Vec<hexagonal_backend::hexagon::domain::volatility::ConstantMaturityVolatilityPoint>,
    > {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn composes_index_and_option_data_through_mocked_driven_ports() {
    let previous = NaiveDate::from_ymd_opt(2026, 7, 31).expect("valid date");
    let current = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid date");
    let histories = [
        ("VIX", vec![(previous, 19.0), (current, 20.0)]),
        ("VVIX", vec![(current, 95.0)]),
        ("VIX9D", vec![(current, 18.0)]),
    ]
    .into_iter()
    .map(|(ticker, values)| {
        (
            ticker.to_string(),
            IndexHistory {
                ticker: ticker.to_string(),
                daily_prices: values
                    .into_iter()
                    .map(|(date, close)| DailyIndexPrice {
                        date,
                        open: None,
                        high: None,
                        low: None,
                        close,
                    })
                    .collect(),
            },
        )
    })
    .collect();
    let term = TermStructure {
        ticker: "SPX".to_string(),
        snapshot_timestamp: current
            .and_hms_opt(20, 15, 0)
            .expect("valid time")
            .and_utc(),
        treasury_date: current,
        points: vec![TermStructurePoint {
            days: 30.0,
            variance: 0.0441,
            volatility: 21.0,
            source: TermStructureSource::Expiration {
                expiration: current,
                interest_rate: 0.04,
            },
        }],
    };
    let application = MarketVolatilityApplication::new(
        IndexHistoryMock(histories),
        OptionDataMock(term),
        MarketHistoryMock,
    );

    let overview = application.volatility_overview().await.expect("overview");

    assert_eq!(overview.as_of, current);
    assert_eq!(overview.vix.close, 20.0);
    assert!(overview.vix.daily_change_percent.unwrap() > 5.0);
    assert_eq!(
        overview
            .spx_30_day
            .expect("30-day point")
            .difference_from_vix,
        1.0
    );
    assert_eq!(overview.term_structure.len(), 1);

    let historical = application.historical_volatility(" spy ").await.unwrap();
    assert_eq!(historical.ticker, "SPY");
    assert!(historical.points.is_empty());

    let implied = application.implied_volatility(" spx ").await.unwrap();
    assert_eq!(implied.reference_ticker.as_deref(), Some("VIX"));
    assert_eq!(implied.points.last().unwrap().volatility_percent, 20.0);
}
