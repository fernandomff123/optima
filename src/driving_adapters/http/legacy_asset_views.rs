//! Compatibility mapping for pre-hexagonal asset-volatility responses.

use api_models::{
    AssetHistoricalVolatilityResponse, AssetImpliedVolatilityResponse, DataState, Freshness,
    HistoricalVolatilityAnalysis, HistoricalVolatilityHorizon, HistoricalVolatilityLatest,
    HistoricalVolatilityOverview as HistoricalVolatilityView, HistoricalVolatilityPoint,
    HistoricalVolatilitySeriesPoint, HistoricalVolatilityStatus,
    ImpliedVolatilityOverview as ImpliedVolatilityView,
    ImpliedVolatilityPoint as ImpliedVolatilityViewPoint, ViewMetadata,
};

use crate::hexagon::domain::{
    historical_volatility::HistoricalVolatilityOverview,
    market_volatility::ImpliedVolatilityOverview,
};

pub fn historical_volatility(
    overview: HistoricalVolatilityOverview,
) -> AssetHistoricalVolatilityResponse {
    let analysis = HistoricalVolatilityAnalysis {
        methodology: overview.methodology.clone(),
        annualization_sessions: overview.annualization_sessions,
        unit: overview.unit.clone(),
        price_basis: overview.price_basis.clone(),
        valid_prices: overview.valid_prices,
        first_valid_observation: overview.first_valid_observation,
        last_valid_observation: overview.last_valid_observation,
        ignored_observations: overview.ignored_observations,
        diagnostics: overview.diagnostics.clone(),
        horizons: overview.horizons.iter().map(map_horizon).collect(),
    };
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
        analysis,
    }
}

fn map_horizon(
    value: &crate::hexagon::domain::historical_volatility::HistoricalVolatilityHorizon,
) -> HistoricalVolatilityHorizon {
    HistoricalVolatilityHorizon {
        window_sessions: value.window_sessions,
        required_prices: value.required_prices,
        status: match value.status {
            crate::hexagon::domain::historical_volatility::HistoricalVolatilityStatus::Available => HistoricalVolatilityStatus::Available,
            crate::hexagon::domain::historical_volatility::HistoricalVolatilityStatus::InsufficientHistory => HistoricalVolatilityStatus::InsufficientHistory,
            crate::hexagon::domain::historical_volatility::HistoricalVolatilityStatus::NoValidPrices => HistoricalVolatilityStatus::NoValidPrices,
            crate::hexagon::domain::historical_volatility::HistoricalVolatilityStatus::InvalidData => HistoricalVolatilityStatus::InvalidData,
            crate::hexagon::domain::historical_volatility::HistoricalVolatilityStatus::NumericFailure => HistoricalVolatilityStatus::NumericFailure,
        },
        latest: value.latest.as_ref().map(|latest| HistoricalVolatilityLatest {
            date: latest.date,
            observations: latest.observations,
            annualized_volatility_percent: latest.annualized_volatility_percent,
        }),
        series: value.series.iter().map(|point| HistoricalVolatilitySeriesPoint {
            date: point.date,
            window_sessions: point.window_sessions,
            annualized_volatility_percent: point.annualized_volatility_percent,
        }).collect(),
        series_truncated: value.series_truncated,
        diagnostics: value.diagnostics.clone(),
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
