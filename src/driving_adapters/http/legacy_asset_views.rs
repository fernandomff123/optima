//! Compatibility mapping for pre-hexagonal asset-volatility responses.

use api_models::{
    AssetHistoricalVolatilityResponse, AssetImpliedVolatilityResponse, DataState, Freshness,
    HistoricalVolatilityOverview as HistoricalVolatilityView, HistoricalVolatilityPoint,
    HistoricalVolatilitySeriesPoint, ImpliedVolatilityOverview as ImpliedVolatilityView,
    ImpliedVolatilityPoint as ImpliedVolatilityViewPoint, ViewMetadata,
};

use crate::hexagon::domain::{
    historical_volatility::HistoricalVolatilityOverview,
    market_volatility::ImpliedVolatilityOverview,
};

pub fn historical_volatility(
    overview: HistoricalVolatilityOverview,
) -> AssetHistoricalVolatilityResponse {
    let historical_volatility = overview
        .as_of
        .filter(|_| !overview.points.is_empty())
        .map(|date| {
            DataState::Available(HistoricalVolatilityView {
                metadata: metadata(date, "Yahoo Finance"),
                points: overview
                    .points
                    .into_iter()
                    .map(|value| HistoricalVolatilityPoint {
                        window_sessions: value.window_sessions,
                        observations: value.observations,
                        annualized_volatility_percent: value.annualized_volatility_percent,
                    })
                    .collect(),
                series: overview
                    .series
                    .into_iter()
                    .map(|value| HistoricalVolatilitySeriesPoint {
                        date: value.date,
                        window_sessions: value.window_sessions,
                        annualized_volatility_percent: value.annualized_volatility_percent,
                    })
                    .collect(),
            })
        })
        .unwrap_or(DataState::Unavailable);
    AssetHistoricalVolatilityResponse {
        ticker: overview.ticker,
        as_of: overview.as_of,
        historical_volatility,
    }
}

pub fn implied_volatility(overview: ImpliedVolatilityOverview) -> AssetImpliedVolatilityResponse {
    let implied_volatility = overview
        .points
        .last()
        .map(|latest| {
            let source = overview
                .reference_ticker
                .as_ref()
                .map(|ticker| format!("CBOE · {ticker}"))
                .unwrap_or_else(|| "CBOE snapshot + U.S. Treasury".to_string());
            DataState::Available(ImpliedVolatilityView {
                metadata: metadata(latest.date, &source),
                reference_ticker: overview.reference_ticker.clone(),
                points: overview
                    .points
                    .iter()
                    .map(|point| ImpliedVolatilityViewPoint {
                        date: point.date,
                        volatility_percent: point.volatility_percent,
                    })
                    .collect(),
            })
        })
        .unwrap_or(DataState::Unavailable);
    AssetImpliedVolatilityResponse {
        ticker: overview.ticker,
        implied_volatility,
    }
}

fn metadata(session_date: chrono::NaiveDate, source: &str) -> ViewMetadata {
    ViewMetadata {
        session_date,
        collected_at: None,
        source: source.to_string(),
        freshness: Freshness::Current,
    }
}
