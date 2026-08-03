//! Time-based driving adapter for end-of-day synchronization.
//!
//! This adapter decides *when* to drive the application. It knows neither
//! provider clients nor persistence adapters; all work crosses the provided
//! `ForSynchronizingMarketData` interface.

use chrono::{DateTime, Datelike, NaiveDate, Utc};

use crate::hexagon::{
    PortResult,
    driving_ports::for_synchronizing_market_data::{
        ForSynchronizingMarketData, SynchronizationFailure, SynchronizeTrackedTickers,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndOfDayRequest {
    pub market_close: DateTime<Utc>,
    pub history_since: NaiveDate,
    pub volatility_indices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndOfDayReport {
    pub items_obtained: usize,
    pub items_stored: u64,
    pub failures: Vec<SynchronizationFailure>,
}

/// Runs one deterministic EOD cycle against a provided application interface.
pub async fn synchronize_end_of_day(
    application: &impl ForSynchronizingMarketData,
    request: EndOfDayRequest,
) -> PortResult<EndOfDayReport> {
    let tracked = application
        .synchronize_tracked_tickers(SynchronizeTrackedTickers {
            since: request.history_since,
            market_close: request.market_close,
        })
        .await?;
    let mut report = EndOfDayReport {
        items_obtained: tracked.items_obtained,
        items_stored: tracked.items_stored,
        failures: tracked.failures,
    };

    for ticker in request.volatility_indices {
        match application.synchronize_volatility_index(&ticker).await {
            Ok(result) => {
                report.items_obtained += result.items_obtained;
                report.items_stored += result.items_stored;
            }
            Err(error) => report.failures.push(SynchronizationFailure {
                ticker,
                operation: "volatility_index".to_string(),
                error: error.to_string(),
            }),
        }
    }

    let year = request.market_close.year();
    match application.synchronize_yield_curves(year).await {
        Ok(result) => {
            report.items_obtained += result.items_obtained;
            report.items_stored += result.items_stored;
        }
        Err(error) => report.failures.push(SynchronizationFailure {
            ticker: year.to_string(),
            operation: "yield_curves".to_string(),
            error: error.to_string(),
        }),
    }
    Ok(report)
}
