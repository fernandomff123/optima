use chrono::{TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::duckdb::portfolio::DuckDbPortfolioAdapter,
    hexagon::{
        domain::portfolio::{
            CashMovement, CashMovementKind, Currency, Money, Portfolio, PortfolioEvent, decimal,
        },
        driven_ports::{
            for_loading_portfolios::ForLoadingPortfolios,
            for_storing_portfolios::ForStoringPortfolios,
        },
    },
};
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn assert_contract(adapter: &(impl ForLoadingPortfolios + ForStoringPortfolios)) {
    let mut portfolio =
        Portfolio::new("main", "Principal", Currency::eur()).expect("portfolio must be valid");
    portfolio
        .record(PortfolioEvent::CashMovement(CashMovement {
            id: "deposit-1".to_string(),
            occurred_at: Utc
                .with_ymd_and_hms(2026, 7, 17, 9, 0, 0)
                .single()
                .expect("date must be valid"),
            kind: CashMovementKind::Deposit,
            amount: Money::new(
                decimal("1000.25").expect("amount must be valid"),
                Currency::eur(),
            ),
        }))
        .expect("event must be valid");

    adapter
        .store_portfolio(&portfolio)
        .await
        .expect("portfolio must store");
    adapter
        .store_portfolio(&portfolio)
        .await
        .expect("repeated storage must be idempotent");

    let loaded = adapter
        .load_portfolio("main")
        .await
        .expect("portfolio must load")
        .expect("portfolio must exist");
    assert_eq!(loaded, portfolio);
}

#[tokio::test]
async fn duckdb_satisfies_portfolio_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-portfolios-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbPortfolioAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}
