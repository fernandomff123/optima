//! Compatibility mapping for the pre-hexagonal market HTTP contract.
//!
//! These functions know transport DTOs but perform no I/O. They remain at the
//! driving edge until clients migrate to the native hexagonal routes.

use api_models::{
    BenchmarkOverview, DataState, Freshness, IndexHistoryOverview, IndexHistoryPoint, IndexValue,
    MarketBenchmarkResponse, MarketRatesResponse, MarketSpxHistoryResponse,
    MarketVixHistoryResponse, MarketVolatilityResponse, PriceHistoryOverview, PriceHistoryPoint,
    RatesOverview, ViewMetadata, VolatilityOverview,
};
use chrono::NaiveDate;

use crate::hexagon::domain::{
    index_history::IndexHistory,
    market_history::MarketHistory,
    market_volatility::{MarketVolatilityOverview, VolatilityIndexValue},
};
use crate::hexagon::driving_ports::for_viewing_interest_rates::InterestRateCurveProjection;

const HISTORY_SESSIONS: usize = 1_260;

pub fn benchmark(history: &MarketHistory, as_of: NaiveDate) -> MarketBenchmarkResponse {
    let (benchmark, _) = market_history_views(history, as_of, "SPY");
    MarketBenchmarkResponse { as_of, benchmark }
}

pub fn spx_history(history: &MarketHistory, as_of: NaiveDate) -> MarketSpxHistoryResponse {
    let (_, spx_history) = market_history_views(history, as_of, "SPX");
    MarketSpxHistoryResponse { as_of, spx_history }
}

pub fn vix_history(history: &IndexHistory, as_of: NaiveDate) -> MarketVixHistoryResponse {
    let vix_history = match history.daily_prices.last() {
        None => DataState::Unavailable,
        Some(latest) => {
            let freshness = freshness(latest.date, as_of);
            let start = history.daily_prices.len().saturating_sub(HISTORY_SESSIONS);
            state(
                IndexHistoryOverview {
                    metadata: metadata(latest.date, "CBOE", freshness),
                    ticker: history.ticker.clone(),
                    points: history.daily_prices[start..]
                        .iter()
                        .map(|point| IndexHistoryPoint {
                            date: point.date,
                            close: point.close,
                        })
                        .collect(),
                },
                freshness,
            )
        }
    };
    MarketVixHistoryResponse { as_of, vix_history }
}

pub fn volatility(overview: MarketVolatilityOverview) -> MarketVolatilityResponse {
    let as_of = overview.as_of;
    MarketVolatilityResponse {
        as_of,
        volatility: DataState::Available(VolatilityOverview {
            metadata: metadata(as_of, "CBOE", Freshness::Current),
            vix: index_value(overview.vix),
            spx_30_day: overview
                .spx_30_day
                .map(|value| api_models::CalculatedVolatility {
                    ticker: value.ticker,
                    snapshot_timestamp: value.snapshot_timestamp,
                    volatility_percent: value.volatility_percent,
                    difference_from_vix: value.difference_from_vix,
                }),
            vvix: overview.vvix.map(index_value),
            term_structure: overview
                .term_structure
                .into_iter()
                .map(index_value)
                .collect(),
        }),
    }
}

pub fn rates(as_of: NaiveDate, curve: Option<InterestRateCurveProjection>) -> MarketRatesResponse {
    let rates = match curve {
        None => DataState::Unavailable,
        Some(curve) => {
            let freshness = freshness(curve.date, as_of);
            state(
                RatesOverview {
                    metadata: metadata(curve.date, "U.S. Treasury", freshness),
                    points: curve
                        .published_points
                        .into_iter()
                        .map(|point| api_models::RatePoint {
                            tenor: point.tenor,
                            days: point.days,
                            rate_percent: point.rate_percent,
                        })
                        .collect(),
                    interpolated_points: curve
                        .interpolated_points
                        .into_iter()
                        .map(|point| api_models::InterpolatedRatePoint {
                            days: point.days,
                            rate_percent: point.rate_percent,
                        })
                        .collect(),
                },
                freshness,
            )
        }
    };
    MarketRatesResponse { as_of, rates }
}

fn market_history_views(
    history: &MarketHistory,
    as_of: NaiveDate,
    ticker: &str,
) -> (
    DataState<BenchmarkOverview>,
    DataState<PriceHistoryOverview>,
) {
    let complete = history
        .daily_quotes
        .iter()
        .filter_map(|quote| {
            Some(PriceHistoryPoint {
                date: quote.timestamp.date_naive(),
                open: quote.open?,
                high: quote.high?,
                low: quote.low?,
                close: quote.close?,
            })
        })
        .collect::<Vec<_>>();
    let Some(latest) = complete.last() else {
        return (DataState::Unavailable, DataState::Unavailable);
    };
    let daily_change_pct = complete
        .get(complete.len().saturating_sub(2))
        .filter(|previous| previous.close != 0.0)
        .map(|previous| (latest.close / previous.close - 1.0) * 100.0);
    let freshness = freshness(latest.date, as_of);
    let view_metadata = metadata(latest.date, "Yahoo Finance", freshness);
    let benchmark = BenchmarkOverview {
        metadata: view_metadata.clone(),
        ticker: ticker.to_string(),
        close: latest.close,
        daily_change_pct,
    };
    let start = complete.len().saturating_sub(HISTORY_SESSIONS);
    let prices = PriceHistoryOverview {
        metadata: view_metadata,
        points: complete[start..].to_vec(),
    };
    (state(benchmark, freshness), state(prices, freshness))
}

fn index_value(value: VolatilityIndexValue) -> IndexValue {
    IndexValue {
        ticker: value.ticker,
        date: value.date,
        close: value.close,
        daily_change_pct: value.daily_change_percent,
    }
}

fn freshness(date: NaiveDate, as_of: NaiveDate) -> Freshness {
    if date >= as_of {
        Freshness::Current
    } else {
        Freshness::Stale
    }
}

fn state<T>(value: T, freshness: Freshness) -> DataState<T> {
    match freshness {
        Freshness::Current => DataState::Available(value),
        Freshness::Stale => DataState::Stale(value),
    }
}

fn metadata(session_date: NaiveDate, source: &str, freshness: Freshness) -> ViewMetadata {
    ViewMetadata {
        session_date,
        collected_at: None,
        source: source.to_string(),
        freshness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hexagon::driving_ports::for_viewing_interest_rates::{
        InterpolatedInterestRatePoint, PublishedInterestRatePoint,
    };

    #[test]
    fn rates_mapping_preserves_the_complete_legacy_json_and_360_point_order() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid date");
        let projection = InterestRateCurveProjection {
            date,
            published_points: vec![
                PublishedInterestRatePoint {
                    tenor: "1M".to_string(),
                    days: 30.0,
                    rate_percent: 4.0,
                },
                PublishedInterestRatePoint {
                    tenor: "3M".to_string(),
                    days: 91.0,
                    rate_percent: 4.1,
                },
            ],
            interpolated_points: (1..=360)
                .map(|month| InterpolatedInterestRatePoint {
                    days: f64::from(month) * 30.0,
                    rate_percent: 4.0,
                })
                .collect(),
        };
        let response = rates(date, Some(projection));
        let expected_interpolated: Vec<_> = (1..=360)
            .map(|month| {
                serde_json::json!({
                    "days": f64::from(month) * 30.0,
                    "rate_percent": 4.0
                })
            })
            .collect();

        assert_eq!(
            serde_json::to_value(response).expect("response must serialize"),
            serde_json::json!({
                "as_of": "2026-08-03",
                "rates": {
                    "state": "available",
                    "data": {
                        "metadata": {
                            "session_date": "2026-08-03",
                            "collected_at": null,
                            "source": "U.S. Treasury",
                            "freshness": "current"
                        },
                        "points": [
                            {"tenor": "1M", "days": 30.0, "rate_percent": 4.0},
                            {"tenor": "3M", "days": 91.0, "rate_percent": 4.1}
                        ],
                        "interpolated_points": expected_interpolated
                    }
                }
            })
        );
    }

    #[test]
    fn absent_curve_preserves_the_legacy_unavailable_json() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 3).expect("valid date");
        assert_eq!(
            serde_json::to_value(rates(date, None)).expect("response must serialize"),
            serde_json::json!({
                "as_of": "2026-08-03",
                "rates": {"state": "unavailable"}
            })
        );
    }
}
