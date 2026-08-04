use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use hexagonal_backend::{
    driving_adapters::scheduler::{EndOfDayRequest, synchronize_end_of_day},
    hexagon::{
        PortError, PortResult,
        driving_ports::for_synchronizing_market_data::{
            ForSynchronizingMarketData, SynchronizationReport, SynchronizeTrackedTickers,
            TrackedTickersSynchronizationReport,
        },
    },
};

struct SynchronizationMock;

#[async_trait]
impl ForSynchronizingMarketData for SynchronizationMock {
    async fn synchronize_tracked_tickers(
        &self,
        _request: SynchronizeTrackedTickers,
    ) -> PortResult<TrackedTickersSynchronizationReport> {
        Ok(TrackedTickersSynchronizationReport {
            tickers: 2,
            items_obtained: 3,
            items_stored: 2,
            failures: Vec::new(),
        })
    }

    async fn synchronize_market_history(
        &self,
        _ticker: &str,
        _since: chrono::NaiveDate,
    ) -> PortResult<SynchronizationReport> {
        unreachable!("the scheduler uses the batch conversation")
    }

    async fn synchronize_option_chain(
        &self,
        _ticker: &str,
        _market_close: chrono::DateTime<Utc>,
    ) -> PortResult<SynchronizationReport> {
        unreachable!("the scheduler uses the batch conversation")
    }

    async fn synchronize_term_structure(&self, _ticker: &str) -> PortResult<SynchronizationReport> {
        unreachable!("the scheduler uses the batch conversation")
    }

    async fn synchronize_volatility_index(
        &self,
        ticker: &str,
    ) -> PortResult<SynchronizationReport> {
        if ticker == "BROKEN" {
            return Err(PortError::Unavailable("recorded failure".to_string()));
        }
        Ok(SynchronizationReport {
            items_obtained: 4,
            items_stored: 4,
        })
    }

    async fn synchronize_yield_curves(&self, year: i32) -> PortResult<SynchronizationReport> {
        assert_eq!(year, 2026);
        Ok(SynchronizationReport {
            items_obtained: 5,
            items_stored: 5,
        })
    }
}

#[tokio::test]
async fn scheduler_drives_ports_and_aggregates_independent_failures() {
    let market_close = Utc
        .with_ymd_and_hms(2026, 8, 3, 20, 0, 0)
        .single()
        .expect("valid timestamp");
    let report = synchronize_end_of_day(
        &SynchronizationMock,
        EndOfDayRequest {
            market_close,
            history_since: market_close.date_naive(),
            volatility_indices: vec!["VIX".to_string(), "BROKEN".to_string()],
        },
    )
    .await
    .expect("the scheduler aggregates individual failures");

    assert_eq!(report.items_obtained, 12);
    assert_eq!(report.items_stored, 11);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].operation, "volatility_index");
}
