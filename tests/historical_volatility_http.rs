use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use chrono::{Duration, TimeZone, Utc};
use hexagonal_backend::{
    configurator::{CompositionConfig, configure_http_application, configure_with_config},
    driven_adapters::duckdb::market_history::DuckDbMarketHistoryAdapter,
    hexagon::{
        domain::market_history::{DailyQuote, MarketHistory},
        driven_ports::for_storing_market_history::ForStoringMarketHistory,
    },
};
use tower::ServiceExt;

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

async fn test_router() -> (axum::Router, std::path::PathBuf) {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "historical-volatility-http-{}-{sequence}.duckdb",
        std::process::id()
    ));
    let store = DuckDbMarketHistoryAdapter::new(&path);
    store.initialize().await.unwrap();
    let history = MarketHistory {
        ticker: "SPX".to_string(),
        currency: Some("USD".to_string()),
        exchange_timezone: Some("America/New_York".to_string()),
        daily_quotes: (0..63)
            .map(|day| DailyQuote {
                timestamp: Utc.with_ymd_and_hms(2026, 1, 1, 21, 0, 0).unwrap()
                    + Duration::days(day),
                open: None,
                high: None,
                low: None,
                close: Some(100.0 + day as f64 * 0.1 + (day % 3) as f64),
                adjusted_close: None,
                volume: None,
            })
            .collect(),
        dividends: Vec::new(),
        splits: Vec::new(),
    };
    store.store_market_history(&history).await.unwrap();
    let config = CompositionConfig::with_duckdb_path(&path);
    let router = configure_http_application(configure_with_config(&config));
    (router, path)
}

async fn get(router: axum::Router, uri: &str) -> axum::response::Response {
    router
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn historical_volatility_http_preserves_legacy_fields_and_returns_partial_states() {
    let (router, path) = test_router().await;
    let response = get(
        router.clone(),
        "/api/assets/SPX/historical-volatility?horizons=252,10&limit=2",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = std::str::from_utf8(&body).unwrap();
    assert!(!text.contains("NaN") && !text.contains("Infinity"));
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("ticker").is_some());
    assert!(json.get("as_of").is_some());
    assert!(json.get("historical_volatility").is_some());
    assert!(json.get("analysis").is_some());
    assert_eq!(json["analysis"]["annualization_sessions"], 252);
    assert_eq!(json["analysis"]["unit"], "percent_annualized");
    assert_eq!(json["analysis"]["horizons"][0]["window_sessions"], 10);
    assert_eq!(json["analysis"]["horizons"][0]["status"], "available");
    assert!(
        json["analysis"]["horizons"][0]["series"]
            .as_array()
            .unwrap()
            .len()
            <= 2
    );
    assert_eq!(json["analysis"]["horizons"][1]["window_sessions"], 252);
    assert_eq!(
        json["analysis"]["horizons"][1]["status"],
        "insufficient_history"
    );
    assert_eq!(json["as_of"], json["analysis"]["last_valid_observation"]);

    drop(router);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn historical_volatility_http_defaults_and_invalid_queries_are_json() {
    let (router, path) = test_router().await;
    let response = get(router.clone(), "/api/assets/SPX/historical-volatility").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json["analysis"]["horizons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value["window_sessions"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![10, 20, 60]
    );

    let response = get(
        router.clone(),
        "/api/assets/SPX/historical-volatility?horizons=60%2C%2020%2C10&limit=%202%20",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        json["analysis"]["horizons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value["window_sessions"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![10, 20, 60]
    );
    assert!(
        json["analysis"]["horizons"]
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value["series"].as_array().unwrap().len() <= 2)
    );

    for query in [
        "horizons=",
        "horizons=abc",
        "horizons=1",
        "horizons=253",
        "horizons=2,3,4,5,6,7,8",
        "horizons=10,10",
        "limit=2&limit=3",
        "unknown=1",
        "limit=0",
        "limit=1261",
    ] {
        let response = get(
            router.clone(),
            &format!("/api/assets/SPX/historical-volatility?{query}"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(
            response.headers()[CONTENT_TYPE],
            "application/json",
            "{query}"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice::<api_models::ApiError>(&body).unwrap();
    }

    drop(router);
    std::fs::remove_file(path).unwrap();
}
