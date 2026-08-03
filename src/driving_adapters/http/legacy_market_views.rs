//! Compatibility mapping for the pre-hexagonal market HTTP contract.
//!
//! These functions know transport DTOs but perform no I/O. They remain at the
//! driving edge until clients migrate to the native hexagonal routes.

use api_models::{
    BenchmarkOverview, DataState, Freshness, IndexHistoryOverview, IndexHistoryPoint, IndexValue,
    InterpolatedRatePoint, MarketBenchmarkResponse, MarketRatesResponse, MarketSpxHistoryResponse,
    MarketVixHistoryResponse, MarketVolatilityResponse, PriceHistoryOverview, PriceHistoryPoint,
    RatePoint, RatesOverview, ViewMetadata, VolatilityOverview,
};
use chrono::NaiveDate;

use crate::hexagon::domain::{
    index_history::IndexHistory,
    interest_rates::BoundedCubicSpline,
    market_history::MarketHistory,
    market_volatility::{MarketVolatilityOverview, VolatilityIndexValue},
    treasury::YieldCurve,
};

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

pub fn rates(
    as_of: NaiveDate,
    curve: Option<&YieldCurve>,
) -> Result<MarketRatesResponse, crate::hexagon::domain::interest_rates::InterestRateError> {
    let rates = match curve {
        None => DataState::Unavailable,
        Some(curve) => {
            let freshness = freshness(curve.date, as_of);
            state(
                RatesOverview {
                    metadata: metadata(curve.date, "U.S. Treasury", freshness),
                    points: yield_points(curve),
                    interpolated_points: interpolated_yield_points(curve)?,
                },
                freshness,
            )
        }
    };
    Ok(MarketRatesResponse { as_of, rates })
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

fn yield_points(curve: &YieldCurve) -> Vec<RatePoint> {
    [
        ("1M", 30.0, curve.m1),
        ("3M", 91.0, curve.m3),
        ("6M", 182.0, curve.m6),
        ("1Y", 365.0, curve.y1),
        ("2Y", 730.0, curve.y2),
        ("5Y", 1_825.0, curve.y5),
        ("10Y", 3_650.0, curve.y10),
        ("30Y", 10_950.0, curve.y30),
    ]
    .into_iter()
    .filter_map(|(tenor, days, rate)| {
        rate.map(|rate| RatePoint {
            tenor: tenor.to_string(),
            days,
            rate_percent: rate * 100.0,
        })
    })
    .collect()
}

fn interpolated_yield_points(
    curve: &YieldCurve,
) -> Result<Vec<InterpolatedRatePoint>, crate::hexagon::domain::interest_rates::InterestRateError> {
    let spline = BoundedCubicSpline::from_treasury_curve(curve)?;
    (1..=360)
        .map(|month| {
            let days = f64::from(month) * 30.0;
            Ok(InterpolatedRatePoint {
                days,
                rate_percent: spline.bond_equivalent_yield(days)? * 100.0,
            })
        })
        .collect()
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
