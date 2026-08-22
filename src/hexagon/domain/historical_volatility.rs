use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::hexagon::domain::market_history::{DailyQuote, MarketHistory};

pub const ANNUALIZATION_SESSIONS: usize = 252;
pub const METHODOLOGY: &str = "sample standard deviation of log returns, annualized by sqrt(252)";
pub const UNIT: &str = "percent_annualized";
pub const PRICE_BASIS: &str = "adjusted_close_with_close_fallback";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatility {
    pub ticker: String,
    pub date: NaiveDate,
    pub window_sessions: usize,
    pub observations: usize,
    pub annualized_volatility_percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoricalVolatilityStatus {
    Available,
    InsufficientHistory,
    NoValidPrices,
    InvalidData,
    NumericFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatilityHorizon {
    pub window_sessions: usize,
    pub required_prices: usize,
    pub status: HistoricalVolatilityStatus,
    pub latest: Option<HistoricalVolatility>,
    pub series: Vec<HistoricalVolatility>,
    pub series_truncated: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalVolatilityOverview {
    pub ticker: String,
    pub as_of: Option<NaiveDate>,
    pub methodology: String,
    pub annualization_sessions: usize,
    pub unit: String,
    pub price_basis: String,
    pub points: Vec<HistoricalVolatility>,
    pub series: Vec<HistoricalVolatility>,
    pub horizons: Vec<HistoricalVolatilityHorizon>,
    pub valid_prices: usize,
    pub first_valid_observation: Option<NaiveDate>,
    pub last_valid_observation: Option<NaiveDate>,
    pub ignored_observations: usize,
    pub diagnostics: Vec<String>,
}

struct PreparedHistory {
    ticker: String,
    prices: Vec<(DateTime<Utc>, f64)>,
    ignored: usize,
    duplicate_timestamps: bool,
}

pub fn analyze(
    history: &MarketHistory,
    horizons: &[usize],
    series_limit: usize,
) -> HistoricalVolatilityOverview {
    let prepared = prepare(history);
    let mut diagnostics = Vec::new();
    if prepared.duplicate_timestamps {
        diagnostics.push("duplicate_timestamps".to_string());
    }
    if prepared.ignored > 0 {
        diagnostics.push("invalid_prices_ignored".to_string());
    }
    let horizon_results: Vec<_> = horizons
        .iter()
        .map(|&window| analyze_horizon(&prepared, window, series_limit))
        .collect();
    let points = horizon_results
        .iter()
        .filter_map(|result| result.latest.clone())
        .collect();
    let series = horizon_results
        .iter()
        .flat_map(|result| result.series.clone())
        .collect();
    HistoricalVolatilityOverview {
        ticker: prepared.ticker.clone(),
        as_of: horizon_results
            .iter()
            .filter_map(|result| result.latest.as_ref().map(|value| value.date))
            .max(),
        methodology: METHODOLOGY.to_string(),
        annualization_sessions: ANNUALIZATION_SESSIONS,
        unit: UNIT.to_string(),
        price_basis: PRICE_BASIS.to_string(),
        points,
        series,
        horizons: horizon_results,
        valid_prices: prepared.prices.len(),
        first_valid_observation: prepared.prices.first().map(|value| value.0.date_naive()),
        last_valid_observation: prepared.prices.last().map(|value| value.0.date_naive()),
        ignored_observations: prepared.ignored,
        diagnostics,
    }
}

fn analyze_horizon(
    prepared: &PreparedHistory,
    window: usize,
    series_limit: usize,
) -> HistoricalVolatilityHorizon {
    let required_prices = window.saturating_add(1);
    if prepared.duplicate_timestamps {
        return unavailable(
            window,
            required_prices,
            HistoricalVolatilityStatus::InvalidData,
            "duplicate_timestamps",
        );
    }
    if prepared.prices.is_empty() {
        return unavailable(
            window,
            required_prices,
            HistoricalVolatilityStatus::NoValidPrices,
            "no_valid_prices",
        );
    }
    if prepared.prices.len() < required_prices {
        return unavailable(
            window,
            required_prices,
            HistoricalVolatilityStatus::InsufficientHistory,
            "insufficient_history",
        );
    }
    let values: Option<Vec<_>> = (window..prepared.prices.len())
        .map(|end| calculate_at(&prepared.ticker, &prepared.prices, window, end))
        .collect();
    let Some(values) = values else {
        return unavailable(
            window,
            required_prices,
            HistoricalVolatilityStatus::NumericFailure,
            "non_finite_calculation",
        );
    };
    let latest = values.last().cloned();
    let series_truncated = values.len() > series_limit;
    let start = values.len().saturating_sub(series_limit);
    HistoricalVolatilityHorizon {
        window_sessions: window,
        required_prices,
        status: HistoricalVolatilityStatus::Available,
        latest,
        series: values.into_iter().skip(start).collect(),
        series_truncated,
        diagnostics: Vec::new(),
    }
}

fn unavailable(
    window_sessions: usize,
    required_prices: usize,
    status: HistoricalVolatilityStatus,
    diagnostic: &str,
) -> HistoricalVolatilityHorizon {
    HistoricalVolatilityHorizon {
        window_sessions,
        required_prices,
        status,
        latest: None,
        series: Vec::new(),
        series_truncated: false,
        diagnostics: vec![diagnostic.to_string()],
    }
}

fn prepare(history: &MarketHistory) -> PreparedHistory {
    let mut seen = HashSet::new();
    let duplicate_timestamps = history
        .daily_quotes
        .iter()
        .any(|quote| !seen.insert(quote.timestamp));
    let mut prices: Vec<_> = history
        .daily_quotes
        .iter()
        .filter_map(|quote| select_price(quote).map(|price| (quote.timestamp, price)))
        .collect();
    prices.sort_by_key(|value| value.0);
    PreparedHistory {
        ticker: history.ticker.trim().to_ascii_uppercase(),
        ignored: history.daily_quotes.len().saturating_sub(prices.len()),
        prices,
        duplicate_timestamps,
    }
}

fn select_price(quote: &DailyQuote) -> Option<f64> {
    [quote.adjusted_close, quote.close]
        .into_iter()
        .flatten()
        .find(|price| price.is_finite() && *price > 0.0)
}

fn calculate_at(
    ticker: &str,
    prices: &[(DateTime<Utc>, f64)],
    window: usize,
    end: usize,
) -> Option<HistoricalVolatility> {
    if window < 2 || end >= prices.len() {
        return None;
    }
    let prices = &prices[end.checked_sub(window)?..=end];
    let mut returns = Vec::with_capacity(window);
    let mut return_sum = 0.0;
    for pair in prices.windows(2) {
        let ratio = pair[1].1 / pair[0].1;
        if !ratio.is_finite() || ratio <= 0.0 {
            return None;
        }
        let log_return = ratio.ln();
        if !log_return.is_finite() {
            return None;
        }
        return_sum += log_return;
        if !return_sum.is_finite() {
            return None;
        }
        returns.push(log_return);
    }
    let mean = return_sum / returns.len() as f64;
    if !mean.is_finite() {
        return None;
    }
    let mut squared_difference_sum = 0.0;
    for value in &returns {
        let difference = *value - mean;
        if !difference.is_finite() {
            return None;
        }
        let squared_difference = difference * difference;
        if !squared_difference.is_finite() {
            return None;
        }
        squared_difference_sum += squared_difference;
        if !squared_difference_sum.is_finite() {
            return None;
        }
    }
    let variance = squared_difference_sum / (window - 1) as f64;
    if !variance.is_finite() || variance < 0.0 {
        return None;
    }
    let standard_deviation = variance.sqrt();
    if !standard_deviation.is_finite() {
        return None;
    }
    let annualization_factor = (ANNUALIZATION_SESSIONS as f64).sqrt();
    if !annualization_factor.is_finite() {
        return None;
    }
    let annualized = standard_deviation * annualization_factor;
    if !annualized.is_finite() {
        return None;
    }
    let volatility = annualized * 100.0;
    if !volatility.is_finite() {
        return None;
    }
    Some(HistoricalVolatility {
        ticker: ticker.to_string(),
        date: prices.last()?.0.date_naive(),
        window_sessions: window,
        observations: returns.len(),
        annualized_volatility_percent: volatility,
    })
}

pub fn calculate(history: &MarketHistory, window_sessions: usize) -> Option<HistoricalVolatility> {
    analyze(history, &[window_sessions], usize::MAX)
        .horizons
        .into_iter()
        .next()?
        .latest
}

pub fn calculate_series(
    history: &MarketHistory,
    window_sessions: usize,
) -> Vec<HistoricalVolatility> {
    analyze(history, &[window_sessions], usize::MAX)
        .horizons
        .into_iter()
        .next()
        .map(|result| result.series)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};

    use super::*;

    fn history(values: &[(Option<f64>, Option<f64>)]) -> MarketHistory {
        MarketHistory {
            ticker: " spy ".to_string(),
            currency: None,
            exchange_timezone: Some("America/New_York".to_string()),
            daily_quotes: values
                .iter()
                .enumerate()
                .map(|(index, (close, adjusted_close))| DailyQuote {
                    timestamp: Utc.with_ymd_and_hms(2025, 1, 1, 21, 0, 0).unwrap()
                        + Duration::days(index as i64),
                    open: None,
                    high: None,
                    low: None,
                    close: *close,
                    adjusted_close: *adjusted_close,
                    volume: None,
                })
                .collect(),
            dividends: Vec::new(),
            splits: Vec::new(),
        }
    }

    #[test]
    fn every_supported_horizon_requires_n_plus_one_prices_without_partial_warm_up() {
        let horizons = [2, 10, 20, 30, 60, 90, 252];
        for window in horizons {
            let short = history(&vec![(Some(100.0), None); window]);
            assert_eq!(
                analyze(&short, &[window], 252).horizons[0].status,
                HistoricalVolatilityStatus::InsufficientHistory
            );
            let complete = history(&vec![(Some(100.0), None); window + 1]);
            let result = analyze(&complete, &[window], 252);
            assert_eq!(
                result.horizons[0].status,
                HistoricalVolatilityStatus::Available
            );
            assert_eq!(
                result.horizons[0].latest.as_ref().unwrap().observations,
                window
            );
            assert_eq!(result.horizons[0].series.len(), 1);
        }
    }

    #[test]
    fn uses_log_returns_sample_variance_sqrt_252_and_percent_and_latest_matches_series() {
        let input = history(&[
            (Some(100.0), None),
            (Some(102.0), None),
            (Some(101.0), None),
        ]);
        let result = analyze(&input, &[2], 252);
        let returns = [(102.0_f64 / 100.0).ln(), (101.0_f64 / 102.0).ln()];
        let mean = returns.iter().sum::<f64>() / 2.0;
        let expected = (returns
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>())
        .sqrt()
            * 252.0_f64.sqrt()
            * 100.0;
        let horizon = &result.horizons[0];
        assert!(
            (horizon
                .latest
                .as_ref()
                .unwrap()
                .annualized_volatility_percent
                - expected)
                .abs()
                < 1e-12
        );
        assert_eq!(horizon.latest.as_ref(), horizon.series.last());
        assert_eq!(
            result.as_of,
            Some(NaiveDate::from_ymd_opt(2025, 1, 3).unwrap())
        );
    }

    #[test]
    fn prefers_valid_adjusted_close_and_falls_back_to_valid_close() {
        let adjusted = history(&[
            (Some(100.0), Some(50.0)),
            (Some(200.0), Some(50.0)),
            (Some(300.0), Some(50.0)),
        ]);
        assert_eq!(
            calculate(&adjusted, 2)
                .unwrap()
                .annualized_volatility_percent,
            0.0
        );
        let fallback = history(&[
            (Some(50.0), Some(f64::NAN)),
            (Some(50.0), Some(0.0)),
            (Some(50.0), Some(-1.0)),
        ]);
        assert_eq!(
            calculate(&fallback, 2)
                .unwrap()
                .annualized_volatility_percent,
            0.0
        );
    }

    #[test]
    fn invalid_prices_are_ignored_and_non_finite_calculations_are_never_output() {
        let invalid = history(&[
            (None, None),
            (Some(0.0), None),
            (Some(-1.0), None),
            (Some(f64::NAN), None),
            (Some(f64::INFINITY), None),
        ]);
        let result = analyze(&invalid, &[2], 252);
        assert_eq!(result.valid_prices, 0);
        assert_eq!(result.ignored_observations, 5);
        assert_eq!(
            result.horizons[0].status,
            HistoricalVolatilityStatus::NoValidPrices
        );

        let extreme = history(&[
            (Some(1e-300), None),
            (Some(1e300), None),
            (Some(1e-300), None),
        ]);
        let result = analyze(&extreme, &[2], 252);
        assert_eq!(
            result.horizons[0].status,
            HistoricalVolatilityStatus::NumericFailure
        );
        assert!(result.points.is_empty());
    }

    #[test]
    fn sorting_is_deterministic_and_duplicates_are_invalid() {
        let ordered = history(&[
            (Some(100.0), None),
            (Some(101.0), None),
            (Some(103.0), None),
        ]);
        let mut reversed = ordered.clone();
        reversed.daily_quotes.reverse();
        assert_eq!(calculate(&ordered, 2), calculate(&reversed, 2));

        let mut duplicate = ordered.clone();
        duplicate.daily_quotes[1].timestamp = duplicate.daily_quotes[0].timestamp;
        assert_eq!(
            analyze(&duplicate, &[2], 252).horizons[0].status,
            HistoricalVolatilityStatus::InvalidData
        );
    }

    #[test]
    fn every_requested_horizon_remains_present_with_an_explicit_state() {
        let available = analyze(
            &history(&[(Some(100.0), None), (Some(101.0), None), (Some(99.0), None)]),
            &[2, 10],
            252,
        );
        assert_eq!(available.horizons.len(), 2);
        assert_eq!(
            available.horizons[0].status,
            HistoricalVolatilityStatus::Available
        );
        assert_eq!(
            available.horizons[1].status,
            HistoricalVolatilityStatus::InsufficientHistory
        );
        assert_eq!(
            available.horizons[0].latest.as_ref(),
            available.horizons[0].series.last()
        );

        let no_prices = analyze(&history(&[(None, None)]), &[2], 252);
        assert_eq!(
            no_prices.horizons[0].status,
            HistoricalVolatilityStatus::NoValidPrices
        );
        assert!(no_prices.horizons[0].latest.is_none());

        let extreme = analyze(
            &history(&[
                (Some(1e-300), None),
                (Some(1e300), None),
                (Some(1e-300), None),
            ]),
            &[2],
            252,
        );
        assert_eq!(
            extreme.horizons[0].status,
            HistoricalVolatilityStatus::NumericFailure
        );
        assert!(extreme.horizons[0].latest.is_none());
    }

    #[test]
    fn duplicate_integrity_failure_keeps_other_factual_diagnostics() {
        let mut input = history(&[
            (Some(100.0), None),
            (Some(101.0), None),
            (None, None),
            (Some(102.0), None),
        ]);
        input.daily_quotes[1].timestamp = input.daily_quotes[0].timestamp;

        let result = analyze(&input, &[2, 10], 252);

        assert!(
            result
                .horizons
                .iter()
                .all(|horizon| horizon.status == HistoricalVolatilityStatus::InvalidData)
        );
        assert_eq!(
            result.diagnostics,
            vec!["duplicate_timestamps", "invalid_prices_ignored"]
        );
        assert!(result.points.is_empty());
        assert!(result.series.is_empty());
        assert!(result.as_of.is_none());
    }
}
