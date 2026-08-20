use hexagonal_backend::{
    driven_adapters::{
        duckdb::tracked_tickers::DuckDbTrackedTickersAdapter,
        sqlite::{tracked_tickers, tracked_tickers::SqliteTrackedTickersAdapter},
    },
    hexagon::{
        PortError,
        application::tracked_tickers::TrackedTickersApplication,
        domain::tracked_ticker::{
            ResolvedUnderlying, TrackedTicker, TrackedTickerConfiguration, TrackedTickerSource,
            UnderlyingMetadata, UnderlyingResolutionState,
        },
        driven_ports::{
            for_loading_tracked_tickers::ForLoadingTrackedTickers,
            for_resolving_underlying_symbols::{
                ForResolvingUnderlyingSymbols, UnderlyingResolutionError,
            },
            for_storing_tracked_tickers::ForStoringTrackedTickers,
        },
        driving_ports::for_managing_tracked_tickers::ForManagingTrackedTickers,
    },
};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::atomic::{AtomicU64, Ordering};
static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct Resolver;

#[async_trait::async_trait]
impl ForResolvingUnderlyingSymbols for Resolver {
    async fn resolve_underlying(
        &self,
        ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        Ok(ResolvedUnderlying {
            ticker: ticker.to_string(),
            metadata: UnderlyingMetadata::default(),
        })
    }
}

async fn assert_contract(adapter: &(impl ForLoadingTrackedTickers + ForStoringTrackedTickers)) {
    let ticker = TrackedTicker {
        ticker: "QQQ".to_string(),
        source: hexagonal_backend::hexagon::domain::tracked_ticker::TrackedTickerSource::User,
        active: true,
        historical_prices: true,
        option_snapshots: false,
        resolution_state: UnderlyingResolutionState::Resolved,
        validated_at: None,
        metadata: UnderlyingMetadata::default(),
    };
    adapter
        .store_tracked_ticker(&ticker)
        .await
        .expect("ticker must store");
    let mut identity_guard = ticker.clone();
    assert!(
        identity_guard
            .resolve(
                ResolvedUnderlying {
                    ticker: "AAPL".into(),
                    metadata: UnderlyingMetadata::default(),
                },
                chrono::Utc::now(),
            )
            .is_err()
    );
    assert_eq!(identity_guard, ticker);
    adapter
        .store_tracked_ticker(&identity_guard)
        .await
        .expect("rejected identity change must not affect persistence");
    assert!(
        adapter
            .load_active_tickers()
            .await
            .expect("tickers must load")
            .contains(&ticker)
    );
    assert!(
        adapter
            .load_refresh_eligible_tickers()
            .await
            .expect("eligible tickers must load")
            .contains(&ticker)
    );
    let mut inactive = ticker.clone();
    inactive.active = false;
    adapter
        .store_tracked_ticker(&inactive)
        .await
        .expect("ticker must update");
    assert!(
        !adapter
            .load_active_tickers()
            .await
            .expect("tickers must load")
            .iter()
            .any(|item| item.ticker == "QQQ")
    );
    assert_eq!(
        adapter
            .load_tracked_tickers()
            .await
            .expect("complete catalog must load"),
        vec![inactive]
    );
}

async fn assert_resolution_eligibility(
    adapter: &(impl ForLoadingTrackedTickers + ForStoringTrackedTickers),
) {
    let configuration = TrackedTickerConfiguration {
        active: true,
        historical_prices: true,
        option_snapshots: false,
    };
    let pending = TrackedTicker::user("PENDING", configuration).unwrap();
    adapter.store_tracked_ticker(&pending).await.unwrap();
    let mut rejected = TrackedTicker::user("REJECTED", configuration).unwrap();
    rejected.reject();
    adapter.store_tracked_ticker(&rejected).await.unwrap();
    let mut resolved = TrackedTicker::user("RESOLVED", configuration).unwrap();
    resolved
        .resolve(
            ResolvedUnderlying {
                ticker: "RESOLVED".into(),
                metadata: UnderlyingMetadata::default(),
            },
            chrono::Utc::now(),
        )
        .expect("matching resolution must succeed");
    adapter.store_tracked_ticker(&resolved).await.unwrap();
    let mut inactive = resolved.clone();
    inactive.ticker = "INACTIVE".into();
    inactive.active = false;
    adapter.store_tracked_ticker(&inactive).await.unwrap();
    let mut system =
        hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers()[0].clone();
    system.active = false;
    adapter.store_tracked_ticker(&system).await.unwrap();

    let eligible = adapter.load_refresh_eligible_tickers().await.unwrap();
    assert!(eligible.iter().any(|ticker| ticker.ticker == "RESOLVED"));
    assert!(eligible.iter().any(|ticker| ticker.ticker == system.ticker));
    assert!(!eligible.iter().any(|ticker| ticker.ticker == "PENDING"));
    assert!(!eligible.iter().any(|ticker| ticker.ticker == "REJECTED"));
    assert!(!eligible.iter().any(|ticker| ticker.ticker == "INACTIVE"));
}

#[tokio::test]
async fn sqlite_satisfies_tracked_ticker_contract() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("SQLite must open");
    tracked_tickers::initialize(&pool)
        .await
        .expect("schema must initialize");
    let adapter = SqliteTrackedTickersAdapter::new(pool);
    assert_contract(&adapter).await;
    assert_resolution_eligibility(&adapter).await;
}

#[tokio::test]
async fn duckdb_satisfies_tracked_ticker_contract() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-tracked-tickers-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert_contract(&adapter).await;
    assert_resolution_eligibility(&adapter).await;
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}

#[tokio::test]
async fn sqlite_rejects_unknown_persisted_source_and_resolution_state() {
    for (column, value) in [("source", "external"), ("resolution_state", "unknown")] {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        tracked_tickers::initialize(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tracked_tickers
             (ticker, source, active, yahoo_prices, cboe_snapshot, resolution_state)
             VALUES ('BROKEN', 'user', 1, 1, 0, 'pending')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(&format!(
            "UPDATE tracked_tickers SET {column} = ? WHERE ticker = 'BROKEN'"
        ))
        .bind(value)
        .execute(&pool)
        .await
        .unwrap();

        let error = SqliteTrackedTickersAdapter::new(pool)
            .load_tracked_tickers()
            .await
            .expect_err("unknown persisted enum must fail loading");
        assert!(error.to_string().contains(column));
        assert!(error.to_string().contains(value));
    }
}

#[tokio::test]
async fn duckdb_rejects_unknown_persisted_source_and_resolution_state() {
    for (column, value) in [("source", "external"), ("resolution_state", "unknown")] {
        let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "hexagonal-invalid-tracked-tickers-{}-{sequence}.duckdb",
            std::process::id()
        ));
        let adapter = DuckDbTrackedTickersAdapter::new(&path);
        adapter.initialize().await.unwrap();
        {
            let connection = duckdb::Connection::open(&path).unwrap();
            connection
                .execute(
                    "INSERT INTO tracked_tickers
                     (ticker, source, active, historical_prices, option_snapshots,
                      resolution_state)
                     VALUES ('BROKEN', 'user', true, true, false, 'pending')",
                    [],
                )
                .unwrap();
            connection
                .execute(
                    &format!("UPDATE tracked_tickers SET {column} = ? WHERE ticker = 'BROKEN'"),
                    [value],
                )
                .unwrap();
        }

        let error = adapter
            .load_tracked_tickers()
            .await
            .expect_err("unknown persisted enum must fail loading");
        assert!(error.to_string().contains(column));
        assert!(error.to_string().contains(value));
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn duckdb_supports_the_complete_tracked_ticker_lifecycle() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-sector-tickers-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.expect("DuckDB must initialize");
    assert!(adapter.load_tracked_tickers().await.unwrap().is_empty());
    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone(), Resolver);
    application.bootstrap_system_tickers().await.unwrap();
    application.bootstrap_system_tickers().await.unwrap();

    let active = adapter
        .load_active_tickers()
        .await
        .expect("tickers must load");
    for ticker in hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers() {
        let tracked = active
            .iter()
            .find(|item| item.ticker == ticker.ticker)
            .expect("default must exist");
        assert_eq!(tracked, &ticker);
    }
    assert_eq!(active.len(), 14);

    let system = active.iter().find(|ticker| ticker.ticker == "SPX").unwrap();
    application
        .configure_ticker("spx", system.configuration())
        .await
        .expect("identical system configuration must be idempotent");
    let forbidden = TrackedTickerConfiguration {
        active: false,
        ..system.configuration()
    };
    assert_eq!(
        application.configure_ticker("SPX", forbidden).await,
        Err(PortError::Conflict(
            "tracked ticker SPX is protected by the system".into()
        ))
    );
    assert_eq!(
        application
            .configure_ticker(
                "bad ticker",
                TrackedTickerConfiguration {
                    active: true,
                    historical_prices: true,
                    option_snapshots: false,
                },
            )
            .await,
        Err(PortError::InvalidRequest("invalid tracked ticker".into()))
    );

    let enabled = TrackedTickerConfiguration {
        active: true,
        historical_prices: true,
        option_snapshots: false,
    };
    application
        .configure_ticker(" qqq ", enabled)
        .await
        .unwrap();
    assert!(
        adapter
            .load_active_tickers()
            .await
            .unwrap()
            .iter()
            .any(|ticker| ticker.ticker == "QQQ")
    );
    let updated = TrackedTickerConfiguration {
        option_snapshots: true,
        ..enabled
    };
    application.configure_ticker("QQQ", updated).await.unwrap();
    let disabled = TrackedTickerConfiguration {
        active: false,
        ..updated
    };
    application.configure_ticker("QQQ", disabled).await.unwrap();
    assert!(
        !adapter
            .load_active_tickers()
            .await
            .unwrap()
            .iter()
            .any(|ticker| ticker.ticker == "QQQ")
    );
    let stored = adapter.load_tracked_tickers().await.unwrap();
    assert!(
        stored
            .iter()
            .any(|ticker| ticker.ticker == "QQQ" && !ticker.active)
    );
    application.configure_ticker("QQQ", updated).await.unwrap();
    let refreshed = adapter.load_active_tickers().await.unwrap();
    assert!(
        refreshed
            .iter()
            .any(|ticker| ticker.ticker == "QQQ" && ticker.option_snapshots)
    );
    assert_eq!(
        refreshed
            .iter()
            .filter(|ticker| ticker.ticker == "QQQ")
            .count(),
        1
    );
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}

#[tokio::test]
async fn duckdb_migrates_the_legacy_schema_idempotently() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-legacy-tracked-tickers-{}-{sequence}.duckdb",
        std::process::id()
    ));
    {
        let connection = duckdb::Connection::open(&path).expect("DuckDB must open");
        connection
            .execute_batch(
                "CREATE TABLE tracked_tickers (
                    ticker VARCHAR PRIMARY KEY,
                    active BOOLEAN NOT NULL,
                    historical_prices BOOLEAN NOT NULL,
                    option_snapshots BOOLEAN NOT NULL
                );
                INSERT INTO tracked_tickers VALUES ('QQQ', false, true, false);
                INSERT INTO tracked_tickers VALUES ('SPX', false, false, false);",
            )
            .expect("legacy schema must be created");
    }
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.expect("migration must succeed");
    adapter
        .initialize()
        .await
        .expect("migration must be idempotent");

    let catalog = adapter
        .load_tracked_tickers()
        .await
        .expect("catalog must load");
    assert_eq!(catalog.len(), 2);
    let qqq = catalog
        .iter()
        .find(|ticker| ticker.ticker == "QQQ")
        .unwrap();
    assert_eq!(qqq.source, TrackedTickerSource::User);
    assert!(!qqq.active);
    assert_eq!(qqq.resolution_state, UnderlyingResolutionState::Pending);

    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone(), Resolver);
    application.bootstrap_system_tickers().await.unwrap();
    application.bootstrap_system_tickers().await.unwrap();
    let promoted = adapter.load_tracked_tickers().await.unwrap();
    assert_eq!(promoted.len(), 15);
    assert_eq!(
        promoted
            .iter()
            .find(|ticker| ticker.ticker == "SPX")
            .unwrap(),
        &hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers()
            .into_iter()
            .find(|ticker| ticker.ticker == "SPX")
            .unwrap()
    );
    std::fs::remove_file(path).expect("temporary DuckDB must be removable");
}

#[tokio::test]
async fn sqlite_migrates_legacy_users_to_pending_and_system_rows_to_resolved() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE tracked_tickers (
            ticker TEXT PRIMARY KEY NOT NULL,
            source TEXT NOT NULL,
            active INTEGER NOT NULL,
            yahoo_prices INTEGER NOT NULL,
            cboe_snapshot INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tracked_tickers VALUES
         ('OLDUSER', 'user', 1, 1, 0), ('SPX', 'system', 1, 1, 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    tracked_tickers::initialize(&pool).await.unwrap();
    tracked_tickers::initialize(&pool).await.unwrap();
    let adapter = SqliteTrackedTickersAdapter::new(pool);
    let stored = adapter.load_tracked_tickers().await.unwrap();
    assert_eq!(
        stored
            .iter()
            .find(|ticker| ticker.ticker == "OLDUSER")
            .unwrap()
            .resolution_state,
        UnderlyingResolutionState::Pending
    );
    assert_eq!(
        stored
            .iter()
            .find(|ticker| ticker.ticker == "SPX")
            .unwrap()
            .resolution_state,
        UnderlyingResolutionState::Resolved
    );
    assert!(
        adapter
            .load_refresh_eligible_tickers()
            .await
            .unwrap()
            .iter()
            .all(|ticker| ticker.ticker != "OLDUSER")
    );
}

async fn seed_resolution_states(
    adapter: &(impl ForLoadingTrackedTickers + ForStoringTrackedTickers),
) -> chrono::DateTime<chrono::Utc> {
    let configuration = TrackedTickerConfiguration {
        active: true,
        historical_prices: true,
        option_snapshots: false,
    };
    adapter
        .store_tracked_ticker(&TrackedTicker::user("PENDING", configuration).unwrap())
        .await
        .unwrap();
    let validated_at = chrono::DateTime::parse_from_rfc3339("2026-08-20T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let metadata = UnderlyingMetadata {
        currency: Some("USD".into()),
        exchange: Some("NMS".into()),
        timezone: Some("America/New_York".into()),
        instrument_type: Some("EQUITY".into()),
    };
    let mut resolved = TrackedTicker::user("RESOLVED", configuration).unwrap();
    resolved
        .resolve(
            ResolvedUnderlying {
                ticker: "RESOLVED".into(),
                metadata: metadata.clone(),
            },
            validated_at,
        )
        .expect("matching resolution must succeed");
    adapter.store_tracked_ticker(&resolved).await.unwrap();
    let mut rejected = TrackedTicker::user("REJECTED", configuration).unwrap();
    rejected.reject();
    adapter.store_tracked_ticker(&rejected).await.unwrap();
    let mut system = hexagonal_backend::hexagon::domain::tracked_ticker::system_tickers()
        .into_iter()
        .find(|ticker| ticker.ticker == "SPX")
        .unwrap();
    system.validated_at = Some(validated_at);
    system.metadata = metadata;
    adapter.store_tracked_ticker(&system).await.unwrap();
    validated_at
}

fn assert_preserved_resolution_states(
    stored: &[TrackedTicker],
    validated_at: chrono::DateTime<chrono::Utc>,
) {
    assert_eq!(
        stored
            .iter()
            .find(|ticker| ticker.ticker == "PENDING")
            .unwrap()
            .resolution_state,
        UnderlyingResolutionState::Pending
    );
    let resolved = stored
        .iter()
        .find(|ticker| ticker.ticker == "RESOLVED")
        .unwrap();
    assert_eq!(
        resolved.resolution_state,
        UnderlyingResolutionState::Resolved
    );
    assert_eq!(resolved.validated_at, Some(validated_at));
    assert_eq!(resolved.metadata.currency.as_deref(), Some("USD"));
    assert_eq!(
        stored
            .iter()
            .find(|ticker| ticker.ticker == "REJECTED")
            .unwrap()
            .resolution_state,
        UnderlyingResolutionState::Rejected
    );
    let system = stored.iter().find(|ticker| ticker.ticker == "SPX").unwrap();
    assert_eq!(system.resolution_state, UnderlyingResolutionState::Resolved);
    assert_eq!(system.validated_at, Some(validated_at));
    assert_eq!(system.metadata.exchange.as_deref(), Some("NMS"));
}

#[tokio::test]
async fn repeated_duckdb_initialization_and_bootstrap_preserve_resolution_evidence() {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "hexagonal-resolution-idempotence-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let adapter = DuckDbTrackedTickersAdapter::new(&path);
    adapter.initialize().await.unwrap();
    let validated_at = seed_resolution_states(&adapter).await;

    adapter.initialize().await.unwrap();
    adapter.initialize().await.unwrap();
    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone(), Resolver);
    application.bootstrap_system_tickers().await.unwrap();
    application.bootstrap_system_tickers().await.unwrap();

    let stored = adapter.load_tracked_tickers().await.unwrap();
    assert_preserved_resolution_states(&stored, validated_at);
    assert_eq!(
        stored
            .iter()
            .filter(|ticker| ticker.ticker == "SPX")
            .count(),
        1
    );
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn repeated_sqlite_initialization_and_bootstrap_preserve_resolution_evidence() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    tracked_tickers::initialize(&pool).await.unwrap();
    let adapter = SqliteTrackedTickersAdapter::new(pool.clone());
    let validated_at = seed_resolution_states(&adapter).await;

    tracked_tickers::initialize(&pool).await.unwrap();
    tracked_tickers::initialize(&pool).await.unwrap();
    let application = TrackedTickersApplication::new(adapter.clone(), adapter.clone(), Resolver);
    application.bootstrap_system_tickers().await.unwrap();
    application.bootstrap_system_tickers().await.unwrap();

    let stored = adapter.load_tracked_tickers().await.unwrap();
    assert_preserved_resolution_states(&stored, validated_at);
    assert_eq!(
        stored
            .iter()
            .filter(|ticker| ticker.ticker == "SPX")
            .count(),
        1
    );
}
