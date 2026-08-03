use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::hexagon::domain::market_history::MarketHistory;

pub const TRADING_DAYS_PER_YEAR: f64 = 252.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatility {
    pub ticker: String,
    pub date: NaiveDate,
    pub window_sessions: usize,
    pub observations: usize,
    pub annualized_volatility_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatilityOverview {
    pub ticker: String,
    pub as_of: Option<NaiveDate>,
    pub points: Vec<HistoricalVolatility>,
    pub series: Vec<HistoricalVolatility>,
}

pub fn calculate(history: &MarketHistory, window_sessions: usize) -> Option<HistoricalVolatility> {
    let prices = valid_prices(history);
    calculate_at(
        history,
        &prices,
        window_sessions,
        prices.len().checked_sub(1)?,
    )
}

pub fn calculate_series(
    history: &MarketHistory,
    window_sessions: usize,
) -> Vec<HistoricalVolatility> {
    if window_sessions < 2 {
        return Vec::new();
    }
    let prices = valid_prices(history);
    (window_sessions..prices.len())
        .filter_map(|end| calculate_at(history, &prices, window_sessions, end))
        .collect()
}

fn valid_prices(history: &MarketHistory) -> Vec<(chrono::DateTime<chrono::Utc>, f64)> {
    history
        .daily_quotes
        .iter()
        .filter_map(|quote| {
            let price = quote.adjusted_close.or(quote.close)?;
            (price.is_finite() && price > 0.0).then_some((quote.timestamp, price))
        })
        .collect()
}

fn calculate_at(
    history: &MarketHistory,
    prices: &[(chrono::DateTime<chrono::Utc>, f64)],
    window_sessions: usize,
    end: usize,
) -> Option<HistoricalVolatility> {
    if window_sessions < 2 || end >= prices.len() {
        return None;
    }
    let start = end.checked_sub(window_sessions)?;
    let prices = &prices[start..=end];
    let returns: Vec<_> = prices
        .windows(2)
        .map(|pair| (pair[1].1 / pair[0].1).ln())
        .collect();
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let sample_variance = returns
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (returns.len() - 1) as f64;

    Some(HistoricalVolatility {
        ticker: history.ticker.trim().to_ascii_uppercase(),
        date: prices.last()?.0.date_naive(),
        window_sessions,
        observations: returns.len(),
        annualized_volatility_percent: sample_variance.sqrt()
            * TRADING_DAYS_PER_YEAR.sqrt()
            * 100.0,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::hexagon::domain::market_history::DailyQuote;

    fn history(prices: &[(f64, Option<f64>)]) -> MarketHistory {
        MarketHistory {
            ticker: "test".to_string(),
            currency: None,
            exchange_timezone: None,
            daily_quotes: prices
                .iter()
                .enumerate()
                .map(|(index, (close, adjusted_close))| DailyQuote {
                    timestamp: Utc
                        .with_ymd_and_hms(2026, 1, index as u32 + 1, 21, 0, 0)
                        .unwrap(),
                    open: None,
                    high: None,
                    low: None,
                    close: Some(*close),
                    adjusted_close: *adjusted_close,
                    volume: None,
                })
                .collect(),
            dividends: Vec::new(),
            splits: Vec::new(),
        }
    }

    #[test]
    fn calculates_sample_log_return_volatility_and_annualizes_it() {
        let history = history(&[(100.0, None), (102.0, None), (101.0, None), (105.0, None)]);

        let result = calculate(&history, 3).unwrap();
        let returns = [
            (102.0_f64 / 100.0).ln(),
            (101.0_f64 / 102.0).ln(),
            (105.0_f64 / 101.0).ln(),
        ];
        let mean = returns.iter().sum::<f64>() / 3.0;
        let variance = returns
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / 2.0;
        let expected = variance.sqrt() * 252.0_f64.sqrt() * 100.0;

        assert!((result.annualized_volatility_percent - expected).abs() < 1e-12);
        assert_eq!(result.observations, 3);
        assert_eq!(result.ticker, "TEST");
    }

    #[test]
    fn prefers_adjusted_close_and_requires_a_complete_window() {
        let adjusted = history(&[(100.0, Some(50.0)), (50.0, Some(50.0)), (51.0, Some(51.0))]);

        let result = calculate(&adjusted, 2).unwrap();

        assert!(result.annualized_volatility_percent < 25.0);
        assert!(calculate(&adjusted, 3).is_none());
    }

    #[test]
    fn builds_one_rolling_value_for_each_complete_window() {
        let history = history(&[
            (100.0, None),
            (101.0, None),
            (99.0, None),
            (102.0, None),
            (103.0, None),
        ]);

        let series = calculate_series(&history, 2);

        assert_eq!(series.len(), 3);
        assert_eq!(series[0].date, NaiveDate::from_ymd_opt(2026, 1, 3).unwrap());
        assert_eq!(series[2], calculate(&history, 2).unwrap());
    }
}
