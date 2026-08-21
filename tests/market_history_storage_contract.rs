use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::duckdb::market_history::DuckDbMarketHistoryAdapter,
    hexagon::{
        domain::market_history::{DailyQuote, Dividend, MarketHistory, StockSplit},
        driven_ports::{
            for_loading_market_history::ForLoadingMarketHistory,
            for_storing_market_history::ForStoringMarketHistory,
        },
    },
};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn history() -> MarketHistory {
    MarketHistory {
        ticker: "XLK".to_string(),
        currency: Some("USD".to_string()),
        exchange_timezone: Some("America/New_York".to_string()),
        daily_quotes: vec![DailyQuote {
            timestamp: Utc
                .with_ymd_and_hms(2026, 8, 7, 20, 0, 0)
                .single()
                .expect("valid quote time"),
            open: Some(630.0),
            high: Some(632.0),
            low: Some(629.0),
            close: Some(631.0),
            adjusted_close: Some(631.0),
            volume: Some(75_000_000),
        }],
        dividends: vec![Dividend {
            timestamp: Utc
                .with_ymd_and_hms(2026, 6, 20, 13, 30, 0)
                .single()
                .expect("valid dividend time"),
            amount: 1.7,
        }],
        splits: vec![StockSplit {
            timestamp: Utc
                .with_ymd_and_hms(2005, 1, 1, 14, 30, 0)
                .single()
                .expect("valid split time"),
            numerator: 2.0,
            denominator: 1.0,
            ratio: "2:1".to_string(),
        }],
    }
}

async fn assert_contract(adapter: &(impl ForLoadingMarketHistory + ForStoringMarketHistory)) {
    let expected = history();
    assert_eq!(
        adapter
            .store_market_history(&expected)
            .await
            .expect("history must be stored"),
        3
    );
    let loaded = adapter
        .load_market_history(" xlk ")
        .await
        .expect("history must load");
    assert_eq!(loaded.ticker, "XLK");
    assert_eq!(loaded.daily_quotes, expected.daily_quotes);
    assert_eq!(loaded.dividends, expected.dividends);
    assert_eq!(loaded.splits, expected.splits);
}

#[tokio::test]
async fn duckdb_satisfies_market_history_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-market-history-contract-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbMarketHistoryAdapter::new(&path);
    adapter
        .initialize()
        .await
        .expect("DuckDB schema must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
