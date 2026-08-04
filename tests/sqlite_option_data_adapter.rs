use chrono::{NaiveDate, TimeZone, Utc};
use hexagonal_backend::{
    driven_adapters::{
        exchange_calendar::ExchangeTradingCalendarAdapter,
        sqlite::{
            market_history, option_data::SqliteOptionDataAdapter, option_snapshots,
            volatility_term_structures,
        },
    },
    hexagon::{
        application::options::OptionsApplication,
        domain::{
            market_history::{DailyQuote, MarketHistory},
            options::{ContratoOpcao, OptionChain, OptionType, Snapshot},
            volatility::{TermStructure, TermStructurePoint, TermStructureSource},
        },
        driving_ports::for_analyzing_options::ForAnalyzingOptions,
    },
};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn options_application_loads_all_inputs_through_the_sqlite_adapter() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    market_history::initialize(&pool).await.unwrap();
    option_snapshots::initialize(&pool).await.unwrap();
    volatility_term_structures::initialize(&pool).await.unwrap();

    let snapshot_time = Utc.with_ymd_and_hms(2026, 8, 3, 21, 0, 0).unwrap();
    let expiration = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let contract = ContratoOpcao {
        occ_symbol: "TEST260918C00105000".into(),
        option_type: OptionType::Call,
        strike: 105.0,
        expiration,
        bid: 1.0,
        ask: 1.2,
        mid: 1.1,
        spread: 0.2,
        volume: 10.0,
        open_interest: 100.0,
        delta: 0.4,
        gamma: 0.02,
        vega: 0.1,
        theta: -0.03,
        rho: 0.01,
        theo: 1.1,
        implied_volatility: Some(0.25),
    };
    let snapshot = Snapshot {
        ticker: "TEST".into(),
        timestamp_utc: snapshot_time,
        contratos: vec![contract.clone()],
        chains: vec![OptionChain {
            root: "TEST".into(),
            contratos: vec![contract],
        }],
    };
    option_snapshots::save_snapshot(&pool, &snapshot, snapshot_time)
        .await
        .unwrap();
    market_history::insert_incremental(
        &pool,
        &MarketHistory {
            ticker: "TEST".into(),
            currency: Some("USD".into()),
            exchange_timezone: None,
            daily_quotes: vec![DailyQuote {
                timestamp: snapshot_time,
                open: Some(100.0),
                high: Some(101.0),
                low: Some(99.0),
                close: Some(100.0),
                adjusted_close: Some(100.0),
                volume: Some(1_000),
            }],
            dividends: Vec::new(),
            splits: Vec::new(),
        },
    )
    .await
    .unwrap();
    volatility_term_structures::insert(
        &pool,
        &TermStructure {
            ticker: "TEST".into(),
            snapshot_timestamp: snapshot_time,
            treasury_date: snapshot_time.date_naive(),
            points: vec![TermStructurePoint {
                days: 30.0,
                variance: 0.04,
                volatility: 20.0,
                source: TermStructureSource::Expiration {
                    expiration,
                    interest_rate: 0.04,
                },
            }],
        },
    )
    .await
    .unwrap();

    let app = OptionsApplication::new(
        SqliteOptionDataAdapter::new(pool),
        ExchangeTradingCalendarAdapter,
    );

    assert_eq!(app.option_chain("TEST").await.unwrap().contratos.len(), 1);
    assert_eq!(app.term_structure("TEST").await.unwrap().points.len(), 1);
    assert_eq!(
        app.volatility_surface("TEST").await.unwrap().points.len(),
        1
    );
}
