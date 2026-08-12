use api_models::{
    DataState, Freshness, MarketSectorPerformanceResponse, SectorBenchmarkOverview,
    SectorPerformanceOverview, SectorReturnOverview, ViewMetadata,
};

use crate::hexagon::domain::sector_performance::{
    InstrumentPerformance, PerformanceState, SectorPerformanceView,
};

pub fn response(view: SectorPerformanceView) -> MarketSectorPerformanceResponse {
    let benchmark = match view.benchmark {
        PerformanceState::Available(performance) => {
            let freshness = freshness(performance.end_date, view.as_of);
            state(
                SectorBenchmarkOverview {
                    metadata: metadata(&performance, freshness),
                    ticker: performance.ticker,
                    return_percent: performance.return_percent,
                },
                freshness,
            )
        }
        PerformanceState::Unavailable => DataState::Unavailable,
    };
    let sectors = view
        .sectors
        .into_iter()
        .map(|item| {
            let performance = match item.comparison {
                PerformanceState::Available(comparison) => {
                    let freshness = freshness(comparison.performance.end_date, view.as_of);
                    state(
                        SectorReturnOverview {
                            metadata: metadata(&comparison.performance, freshness),
                            return_percent: comparison.performance.return_percent,
                            relative_strength_percentage_points: comparison
                                .relative_strength_percentage_points,
                        },
                        freshness,
                    )
                }
                PerformanceState::Unavailable => DataState::Unavailable,
            };
            SectorPerformanceOverview {
                name: item.sector.name.to_string(),
                etf: item.sector.etf.to_string(),
                performance,
            }
        })
        .collect();
    MarketSectorPerformanceResponse {
        as_of: view.as_of,
        period: match view.period {
            crate::hexagon::domain::sector_performance::SectorPerformancePeriod::OneWeek => {
                api_models::SectorPerformancePeriod::OneWeek
            }
            crate::hexagon::domain::sector_performance::SectorPerformancePeriod::TwoWeeks => {
                api_models::SectorPerformancePeriod::TwoWeeks
            }
            crate::hexagon::domain::sector_performance::SectorPerformancePeriod::OneMonth => {
                api_models::SectorPerformancePeriod::OneMonth
            }
        },
        benchmark,
        sectors,
    }
}

fn freshness(session_date: chrono::NaiveDate, as_of: chrono::NaiveDate) -> Freshness {
    if session_date >= as_of {
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

fn metadata(performance: &InstrumentPerformance, freshness: Freshness) -> ViewMetadata {
    ViewMetadata {
        session_date: performance.end_date,
        // The persisted history records the market observation time but not
        // the provider collection time, so this must not be fabricated.
        collected_at: None,
        source: "Yahoo Finance".to_string(),
        freshness,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, TimeZone, Utc};

    use super::*;
    use crate::hexagon::domain::sector_performance::{
        PerformanceState, SectorPerformancePeriod, SectorPerformanceView,
    };

    fn view(as_of: chrono::NaiveDate, end_date: chrono::NaiveDate) -> SectorPerformanceView {
        SectorPerformanceView {
            as_of,
            period: SectorPerformancePeriod::OneWeek,
            benchmark: PerformanceState::Available(InstrumentPerformance {
                ticker: "SPY".to_string(),
                start_date: end_date - chrono::Duration::days(7),
                end_date,
                return_percent: 1.5,
                observed_at: Utc
                    .with_ymd_and_hms(end_date.year(), end_date.month(), end_date.day(), 21, 0, 0)
                    .unwrap(),
            }),
            sectors: Vec::new(),
        }
    }

    #[test]
    fn maps_current_and_stale_freshness_into_data_state() {
        let as_of = chrono::NaiveDate::from_ymd_opt(2026, 8, 11).unwrap();
        let current = response(view(as_of, as_of));
        let stale = response(view(as_of, as_of - chrono::Duration::days(1)));

        assert!(matches!(current.benchmark, DataState::Available(_)));
        assert!(matches!(stale.benchmark, DataState::Stale(_)));
    }
}
