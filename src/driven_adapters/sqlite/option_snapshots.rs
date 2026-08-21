use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::error::Error;
use std::fmt;

use crate::hexagon::domain::options::{
    OptionChain, OptionIngestionDiagnostics, ProviderTimestamp, Snapshot,
    UnderlyingPriceObservation,
};

const LEGACY_FORMAT_VERSION: i64 = 2;
const CURRENT_FORMAT_VERSION: i64 = 3;

#[derive(Serialize)]
struct SnapshotPayloadRef<'a> {
    chains: &'a [OptionChain],
    underlying_price: &'a Option<UnderlyingPriceObservation>,
    collected_at: Option<DateTime<Utc>>,
    provider_timestamp: &'a Option<ProviderTimestamp>,
    ingestion_diagnostics: &'a OptionIngestionDiagnostics,
}

#[derive(Deserialize)]
struct SnapshotPayload {
    chains: Vec<OptionChain>,
    #[serde(default)]
    underlying_price: Option<UnderlyingPriceObservation>,
    #[serde(default)]
    collected_at: Option<DateTime<Utc>>,
    #[serde(default)]
    provider_timestamp: Option<ProviderTimestamp>,
    #[serde(default)]
    ingestion_diagnostics: OptionIngestionDiagnostics,
}

#[derive(Debug, PartialEq)]
pub enum SnapshotStorageError {
    EmptySnapshot,
    UnsupportedFormat(i64),
}

impl fmt::Display for SnapshotStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySnapshot => write!(formatter, "não é possível guardar um snapshot vazio"),
            Self::UnsupportedFormat(version) => {
                write!(formatter, "versão de snapshot não suportada: {version}")
            }
        }
    }
}

impl Error for SnapshotStorageError {}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    rename_table_if_needed(pool, "cboe_snapshots", "option_snapshots").await?;
    for index in [
        "idx_cboe_snapshots_timestamp",
        "idx_cboe_snapshots_hash",
        "idx_cboe_snapshots_market_close",
    ] {
        sqlx::query(&format!("DROP INDEX IF EXISTS {index}"))
            .execute(pool)
            .await?;
    }
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS option_snapshots (
            ticker TEXT NOT NULL,
            timestamp TIMESTAMP NOT NULL,
            market_close TIMESTAMP,
            format_version INTEGER NOT NULL,
            payload BLOB NOT NULL,
            hash TEXT NOT NULL,
            PRIMARY KEY (ticker, timestamp)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_option_snapshots_timestamp
         ON option_snapshots (timestamp)",
    )
    .execute(pool)
    .await?;

    let columns = sqlx::query("PRAGMA table_info(option_snapshots)")
        .fetch_all(pool)
        .await?;
    let has_hash = columns
        .iter()
        .any(|column| column.get::<String, _>("name") == "hash");
    if !has_hash {
        sqlx::query("ALTER TABLE option_snapshots ADD COLUMN hash TEXT")
            .execute(pool)
            .await?;
    }

    let has_market_close = columns
        .iter()
        .any(|column| column.get::<String, _>("name") == "market_close");
    if !has_market_close {
        sqlx::query("ALTER TABLE option_snapshots ADD COLUMN market_close TIMESTAMP")
            .execute(pool)
            .await?;
    }

    let rows = sqlx::query("SELECT rowid, payload FROM option_snapshots WHERE hash IS NULL")
        .fetch_all(pool)
        .await?;
    for row in rows {
        let rowid: i64 = row.try_get("rowid")?;
        let payload: Vec<u8> = row.try_get("payload")?;
        sqlx::query("UPDATE option_snapshots SET hash = ? WHERE rowid = ?")
            .bind(payload_hash(&payload))
            .bind(rowid)
            .execute(pool)
            .await?;
    }

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_option_snapshots_hash
         ON option_snapshots (hash)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_option_snapshots_market_close
         ON option_snapshots (ticker, market_close)
         WHERE market_close IS NOT NULL",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn rename_table_if_needed(
    pool: &SqlitePool,
    old: &str,
    new: &str,
) -> Result<(), sqlx::Error> {
    let old_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
    )
    .bind(old)
    .fetch_one(pool)
    .await?;
    let new_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
    )
    .bind(new)
    .fetch_one(pool)
    .await?;
    if old_exists && !new_exists {
        sqlx::query(&format!("ALTER TABLE {old} RENAME TO {new}"))
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn save_snapshot(
    pool: &SqlitePool,
    snapshot: &Snapshot,
    market_close: DateTime<Utc>,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    if snapshot.chains.is_empty() {
        return Err(SnapshotStorageError::EmptySnapshot.into());
    }

    let payload = rmp_serde::to_vec(&SnapshotPayloadRef {
        chains: &snapshot.chains,
        underlying_price: &snapshot.underlying_price,
        collected_at: snapshot.collected_at,
        provider_timestamp: &snapshot.provider_timestamp,
        ingestion_diagnostics: &snapshot.ingestion_diagnostics,
    })?;
    let hash = payload_hash(&payload);
    let already_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM option_snapshots WHERE hash = ?)")
            .bind(&hash)
            .fetch_one(pool)
            .await?;
    if already_exists {
        sqlx::query(
            "UPDATE option_snapshots
             SET market_close = ?
             WHERE hash = ?
               AND market_close IS NULL
               AND NOT EXISTS (
                   SELECT 1 FROM option_snapshots
                   WHERE ticker = ? AND market_close = ?
               )",
        )
        .bind(market_close)
        .bind(&hash)
        .bind(snapshot.ticker.trim().to_ascii_uppercase())
        .bind(market_close)
        .execute(pool)
        .await?;
        return Ok(false);
    }

    let result = sqlx::query(
        "INSERT OR IGNORE INTO option_snapshots
         (ticker, timestamp, market_close, format_version, payload, hash)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(snapshot.ticker.trim().to_ascii_uppercase())
    .bind(snapshot.timestamp_utc)
    .bind(market_close)
    .bind(CURRENT_FORMAT_VERSION)
    .bind(payload)
    .bind(hash)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn contains_market_close(
    pool: &SqlitePool,
    ticker: &str,
    market_close: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM option_snapshots
            WHERE ticker = ? AND market_close = ?
        )",
    )
    .bind(ticker.trim().to_ascii_uppercase())
    .bind(market_close)
    .fetch_one(pool)
    .await
}

fn payload_hash(payload: &[u8]) -> String {
    format!("{:x}", Sha256::digest(payload))
}

pub async fn load_latest_at_or_before(
    pool: &SqlitePool,
    ticker: &str,
    target: DateTime<Utc>,
) -> Result<Option<Snapshot>, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query(
        "SELECT ticker, timestamp, format_version, payload
         FROM option_snapshots
         WHERE ticker = ? AND timestamp <= ?
         ORDER BY timestamp DESC
         LIMIT 1",
    )
    .bind(ticker.trim().to_ascii_uppercase())
    .bind(target)
    .fetch_optional(pool)
    .await?;

    row.map(row_to_snapshot).transpose()
}

pub async fn load_latest(
    pool: &SqlitePool,
    ticker: &str,
) -> Result<Option<Snapshot>, Box<dyn Error + Send + Sync>> {
    let row = sqlx::query(
        "SELECT ticker, timestamp, format_version, payload
         FROM option_snapshots
         WHERE ticker = ?
         ORDER BY timestamp DESC
         LIMIT 1",
    )
    .bind(ticker.trim().to_ascii_uppercase())
    .fetch_optional(pool)
    .await?;

    row.map(row_to_snapshot).transpose()
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<Snapshot>, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query(
        "SELECT ticker, timestamp, format_version, payload
         FROM option_snapshots
         ORDER BY timestamp, ticker",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_snapshot).collect()
}

/// Snapshot plus storage metadata required by one-off adapter migrations.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredSnapshot {
    pub snapshot: Snapshot,
    pub market_close: Option<DateTime<Utc>>,
    pub payload_hash: String,
}

/// Reads the legacy representation without exposing its MessagePack payload.
pub async fn load_all_with_metadata(
    pool: &SqlitePool,
) -> Result<Vec<StoredSnapshot>, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query(
        "SELECT ticker, timestamp, market_close, format_version, payload, hash
         FROM option_snapshots
         ORDER BY timestamp, ticker",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let market_close = row.try_get("market_close")?;
            let payload_hash = row.try_get("hash")?;
            Ok(StoredSnapshot {
                snapshot: row_to_snapshot(row)?,
                market_close,
                payload_hash,
            })
        })
        .collect()
}

fn row_to_snapshot(row: sqlx::sqlite::SqliteRow) -> Result<Snapshot, Box<dyn Error + Send + Sync>> {
    let format_version: i64 = row.try_get("format_version")?;
    let payload: Vec<u8> = row.try_get("payload")?;
    let payload = match format_version {
        LEGACY_FORMAT_VERSION => decode_v2_payload(&payload)?,
        CURRENT_FORMAT_VERSION => decode_v3_payload(&payload)?,
        version => return Err(SnapshotStorageError::UnsupportedFormat(version).into()),
    };
    let contratos = payload
        .chains
        .iter()
        .flat_map(|chain| chain.contratos.iter().cloned())
        .collect();

    Ok(Snapshot {
        ticker: row.try_get("ticker")?,
        timestamp_utc: row.try_get("timestamp")?,
        contratos,
        chains: payload.chains,
        underlying_price: payload.underlying_price,
        collected_at: payload.collected_at,
        provider_timestamp: payload.provider_timestamp,
        ingestion_diagnostics: payload.ingestion_diagnostics,
    })
}

fn decode_v2_payload(payload: &[u8]) -> Result<SnapshotPayload, rmp_serde::decode::Error> {
    rmp_serde::from_slice(payload)
}

fn decode_v3_payload(payload: &[u8]) -> Result<SnapshotPayload, rmp_serde::decode::Error> {
    rmp_serde::from_slice(payload)
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;
    use crate::hexagon::domain::options::{ContratoOpcao, OptionType};

    #[derive(Serialize)]
    struct LegacySnapshotPayload<Contract> {
        chains: Vec<LegacyOptionChain<Contract>>,
        underlying_price: Option<UnderlyingPriceObservation>,
        collected_at: Option<DateTime<Utc>>,
        provider_timestamp: Option<ProviderTimestamp>,
        ingestion_diagnostics: OptionIngestionDiagnostics,
    }

    #[derive(Serialize)]
    struct LegacyOptionChain<Contract> {
        root: String,
        contratos: Vec<Contract>,
    }

    #[derive(Serialize)]
    struct LegacyContract {
        occ_symbol: String,
        option_type: OptionType,
        strike: f64,
        expiration: NaiveDate,
        bid: f64,
        ask: f64,
        mid: f64,
        spread: f64,
        volume: f64,
        open_interest: f64,
        delta: f64,
        gamma: f64,
        vega: f64,
        theta: f64,
        rho: f64,
        theo: f64,
        implied_volatility: Option<f64>,
        contract_specification:
            Option<crate::hexagon::domain::options::OptionContractSpecification>,
    }

    #[derive(Serialize)]
    struct ContractWithoutNullableMarketFacts {
        occ_symbol: String,
        option_type: OptionType,
        strike: f64,
        expiration: NaiveDate,
        bid: f64,
        ask: f64,
        mid: f64,
        spread: f64,
        volume: f64,
        delta: f64,
        vega: f64,
        theta: f64,
        rho: f64,
        theo: f64,
        implied_volatility: Option<f64>,
        contract_specification:
            Option<crate::hexagon::domain::options::OptionContractSpecification>,
    }

    async fn memory_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn sample_snapshot() -> Snapshot {
        let contract = ContratoOpcao {
            occ_symbol: "SPY   260717C00500000".to_string(),
            option_type: OptionType::Call,
            strike: 500.0,
            expiration: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            bid: 10.0,
            ask: 10.2,
            mid: 10.1,
            spread: 0.2,
            volume: 100.0,
            open_interest: Some(1_000.0),
            delta: 0.5,
            gamma: Some(0.02),
            vega: 0.15,
            theta: -0.05,
            rho: 0.03,
            theo: 10.1,
            implied_volatility: Some(0.2),
            contract_specification: None,
        };

        Snapshot {
            ticker: "SPY".to_string(),
            timestamp_utc: Utc.with_ymd_and_hms(2026, 7, 13, 15, 0, 0).unwrap(),
            contratos: vec![contract.clone()],
            chains: vec![OptionChain {
                root: "SPY".to_string(),
                contratos: vec![contract],
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: OptionIngestionDiagnostics::default(),
        }
    }

    #[tokio::test]
    async fn saves_and_reconstructs_snapshot() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let expected = sample_snapshot();

        save_snapshot(&pool, &expected, expected.timestamp_utc)
            .await
            .unwrap();
        let loaded = load_latest(&pool, "spy").await.unwrap().unwrap();
        let format_version: i64 = sqlx::query_scalar("SELECT format_version FROM option_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(loaded, expected);
        assert_eq!(format_version, CURRENT_FORMAT_VERSION);
    }

    #[tokio::test]
    async fn preserves_offsetless_timestamp_evidence_without_promoting_it() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let mut expected = sample_snapshot();
        expected.collected_at = Some(Utc.with_ymd_and_hms(2026, 8, 20, 15, 0, 2).unwrap());
        expected.provider_timestamp = Some(ProviderTimestamp {
            raw: "2026-08-20 11:00:00".to_string(),
            timezone: crate::hexagon::domain::options::ProviderTimestampTimezone::Unverified,
        });
        expected.underlying_price =
            UnderlyingPriceObservation::new(500.0, None, None, "cboe_delayed_quotes").map(
                |observation| {
                    observation.with_provider_timestamp(
                        Some("2026-08-20T10:59:58".to_string()),
                        Some(
                            crate::hexagon::domain::options::ProviderTimestampTimezone::Unverified,
                        ),
                    )
                },
            );

        save_snapshot(&pool, &expected, expected.timestamp_utc)
            .await
            .unwrap();
        assert_eq!(load_latest(&pool, "SPY").await.unwrap(), Some(expected));
    }

    #[tokio::test]
    async fn loads_legacy_numeric_gamma_and_open_interest_as_present_values() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let timestamp = Utc.with_ymd_and_hms(2026, 7, 13, 15, 0, 0).unwrap();
        let payload = rmp_serde::to_vec(&LegacySnapshotPayload {
            chains: vec![LegacyOptionChain {
                root: "SPY".to_string(),
                contratos: vec![LegacyContract {
                    occ_symbol: "SPY   260717C00500000".to_string(),
                    option_type: OptionType::Call,
                    strike: 500.0,
                    expiration: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                    bid: 10.0,
                    ask: 10.2,
                    mid: 10.1,
                    spread: 0.2,
                    volume: 100.0,
                    open_interest: 1_000.0,
                    delta: 0.5,
                    gamma: 0.02,
                    vega: 0.15,
                    theta: -0.05,
                    rho: 0.03,
                    theo: 10.1,
                    implied_volatility: Some(0.2),
                    contract_specification: None,
                }],
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: OptionIngestionDiagnostics::default(),
        })
        .unwrap();
        sqlx::query(
            "INSERT INTO option_snapshots
             (ticker, timestamp, market_close, format_version, payload, hash)
             VALUES (?, ?, NULL, ?, ?, ?)",
        )
        .bind("SPY")
        .bind(timestamp)
        .bind(LEGACY_FORMAT_VERSION)
        .bind(&payload)
        .bind(payload_hash(&payload))
        .execute(&pool)
        .await
        .unwrap();

        let loaded = load_latest(&pool, "SPY").await.unwrap().unwrap();
        assert_eq!(loaded.contratos[0].gamma, Some(0.02));
        assert_eq!(loaded.contratos[0].open_interest, Some(1_000.0));
    }

    #[tokio::test]
    async fn loads_v3_named_messagepack_with_missing_gamma_and_open_interest() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let timestamp = Utc.with_ymd_and_hms(2026, 7, 13, 15, 0, 0).unwrap();
        let payload = rmp_serde::to_vec_named(&LegacySnapshotPayload {
            chains: vec![LegacyOptionChain {
                root: "SPY".to_string(),
                contratos: vec![ContractWithoutNullableMarketFacts {
                    occ_symbol: "SPY   260717C00500000".to_string(),
                    option_type: OptionType::Call,
                    strike: 500.0,
                    expiration: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                    bid: 10.0,
                    ask: 10.2,
                    mid: 10.1,
                    spread: 0.2,
                    volume: 100.0,
                    delta: 0.5,
                    vega: 0.15,
                    theta: -0.05,
                    rho: 0.03,
                    theo: 10.1,
                    implied_volatility: Some(0.2),
                    contract_specification: None,
                }],
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: OptionIngestionDiagnostics::default(),
        })
        .unwrap();
        sqlx::query(
            "INSERT INTO option_snapshots
             (ticker, timestamp, market_close, format_version, payload, hash)
             VALUES (?, ?, NULL, ?, ?, ?)",
        )
        .bind("SPY")
        .bind(timestamp)
        .bind(CURRENT_FORMAT_VERSION)
        .bind(&payload)
        .bind(payload_hash(&payload))
        .execute(&pool)
        .await
        .unwrap();

        let loaded = load_latest(&pool, "SPY").await.unwrap().unwrap();
        assert_eq!(loaded.contratos[0].gamma, None);
        assert_eq!(loaded.contratos[0].open_interest, None);
    }

    #[tokio::test]
    async fn v3_round_trip_distinguishes_present_zero_and_missing_values() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let mut snapshot = sample_snapshot();
        let zero = &mut snapshot.chains[0].contratos[0];
        zero.gamma = Some(0.0);
        zero.open_interest = Some(0.0);
        snapshot.contratos[0] = zero.clone();
        let mut missing = zero.clone();
        missing.occ_symbol = "SPY   260717P00500000".to_string();
        missing.option_type = OptionType::Put;
        missing.gamma = None;
        missing.open_interest = None;
        snapshot.chains[0].contratos.push(missing.clone());
        snapshot.contratos.push(missing);

        save_snapshot(&pool, &snapshot, snapshot.timestamp_utc)
            .await
            .unwrap();
        let loaded = load_latest(&pool, "SPY").await.unwrap().unwrap();

        assert_eq!(loaded.contratos[0].gamma, Some(0.0));
        assert_eq!(loaded.contratos[0].open_interest, Some(0.0));
        assert_eq!(loaded.contratos[1].gamma, None);
        assert_eq!(loaded.contratos[1].open_interest, None);
    }

    #[tokio::test]
    async fn loads_v2_and_v3_rows_from_the_same_database() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let legacy_timestamp = Utc.with_ymd_and_hms(2026, 7, 13, 15, 0, 0).unwrap();
        let legacy_payload = rmp_serde::to_vec(&LegacySnapshotPayload {
            chains: vec![LegacyOptionChain {
                root: "SPY".to_string(),
                contratos: vec![LegacyContract {
                    occ_symbol: "SPY   260717C00500000".to_string(),
                    option_type: OptionType::Call,
                    strike: 500.0,
                    expiration: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                    bid: 10.0,
                    ask: 10.2,
                    mid: 10.1,
                    spread: 0.2,
                    volume: 100.0,
                    open_interest: 1_000.0,
                    delta: 0.5,
                    gamma: 0.02,
                    vega: 0.15,
                    theta: -0.05,
                    rho: 0.03,
                    theo: 10.1,
                    implied_volatility: Some(0.2),
                    contract_specification: None,
                }],
            }],
            underlying_price: None,
            collected_at: None,
            provider_timestamp: None,
            ingestion_diagnostics: OptionIngestionDiagnostics::default(),
        })
        .unwrap();
        sqlx::query(
            "INSERT INTO option_snapshots
             (ticker, timestamp, market_close, format_version, payload, hash)
             VALUES (?, ?, NULL, ?, ?, ?)",
        )
        .bind("SPY")
        .bind(legacy_timestamp)
        .bind(LEGACY_FORMAT_VERSION)
        .bind(&legacy_payload)
        .bind(payload_hash(&legacy_payload))
        .execute(&pool)
        .await
        .unwrap();

        let mut current = sample_snapshot();
        current.ticker = "AAPL".to_string();
        current.timestamp_utc += chrono::Duration::days(1);
        current.chains[0].root = "AAPL".to_string();
        current.chains[0].contratos[0].gamma = None;
        current.chains[0].contratos[0].open_interest = None;
        current.contratos = current.chains[0].contratos.clone();
        save_snapshot(&pool, &current, current.timestamp_utc)
            .await
            .unwrap();

        let loaded = load_all(&pool).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].contratos[0].gamma, Some(0.02));
        assert_eq!(loaded[0].contratos[0].open_interest, Some(1_000.0));
        assert_eq!(loaded[1].contratos[0].gamma, None);
        assert_eq!(loaded[1].contratos[0].open_interest, None);
    }

    #[tokio::test]
    async fn loads_all_snapshots() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let first = sample_snapshot();
        let mut second = first.clone();
        second.ticker = "AAPL".to_string();
        second.timestamp_utc += chrono::Duration::days(1);
        second.chains[0].root = "AAPL".to_string();

        save_snapshot(&pool, &first, first.timestamp_utc)
            .await
            .unwrap();
        save_snapshot(&pool, &second, second.timestamp_utc)
            .await
            .unwrap();

        assert_eq!(load_all(&pool).await.unwrap(), vec![first, second]);
    }

    #[tokio::test]
    async fn ignores_a_snapshot_with_an_existing_payload_hash() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let first = sample_snapshot();
        let mut duplicate_payload = first.clone();
        duplicate_payload.timestamp_utc += chrono::Duration::minutes(1);

        assert!(
            save_snapshot(&pool, &first, first.timestamp_utc)
                .await
                .unwrap()
        );
        assert!(
            !save_snapshot(&pool, &duplicate_payload, first.timestamp_utc)
                .await
                .unwrap()
        );

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM option_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn stores_only_one_snapshot_per_market_close() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let first = sample_snapshot();
        let mut same_session = first.clone();
        same_session.timestamp_utc += chrono::Duration::minutes(1);
        same_session.chains[0].contratos[0].bid = 11.0;
        same_session.contratos[0].bid = 11.0;

        assert!(
            save_snapshot(&pool, &first, first.timestamp_utc)
                .await
                .unwrap()
        );
        assert!(
            !save_snapshot(&pool, &same_session, first.timestamp_utc)
                .await
                .unwrap()
        );
        assert!(
            contains_market_close(&pool, "spy", first.timestamp_utc)
                .await
                .unwrap()
        );

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM option_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn migrates_an_existing_table_and_backfills_its_hash() {
        #[derive(Serialize)]
        struct LegacyPayload<'a> {
            chains: &'a [OptionChain],
        }
        let pool = memory_pool().await;
        sqlx::query(
            "CREATE TABLE cboe_snapshots (
                ticker TEXT NOT NULL,
                timestamp TIMESTAMP NOT NULL,
                format_version INTEGER NOT NULL,
                payload BLOB NOT NULL,
                PRIMARY KEY (ticker, timestamp)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        let snapshot = sample_snapshot();
        let payload = rmp_serde::to_vec(&LegacyPayload {
            chains: &snapshot.chains,
        })
        .unwrap();
        sqlx::query(
            "INSERT INTO cboe_snapshots (ticker, timestamp, format_version, payload)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&snapshot.ticker)
        .bind(snapshot.timestamp_utc)
        .bind(LEGACY_FORMAT_VERSION)
        .bind(&payload)
        .execute(&pool)
        .await
        .unwrap();

        initialize(&pool).await.unwrap();

        let hash: String = sqlx::query_scalar("SELECT hash FROM option_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(hash, payload_hash(&payload));
        assert_eq!(hash.len(), 64);
        let loaded = load_latest(&pool, "SPY").await.unwrap().unwrap();
        assert!(loaded.underlying_price.is_none());
        assert!(loaded.collected_at.is_none());
        assert!(loaded.provider_timestamp.is_none());
        assert!(
            loaded.chains[0].contratos[0]
                .contract_specification
                .is_none()
        );
    }

    #[tokio::test]
    async fn repeated_initialization_does_not_rewrite_existing_rows() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let snapshot = sample_snapshot();
        save_snapshot(&pool, &snapshot, snapshot.timestamp_utc)
            .await
            .unwrap();
        let before: (i64, Vec<u8>, String) = sqlx::query_as(
            "SELECT format_version, payload, hash FROM option_snapshots WHERE ticker = ?",
        )
        .bind("SPY")
        .fetch_one(&pool)
        .await
        .unwrap();

        initialize(&pool).await.unwrap();
        initialize(&pool).await.unwrap();

        let after: (i64, Vec<u8>, String) = sqlx::query_as(
            "SELECT format_version, payload, hash FROM option_snapshots WHERE ticker = ?",
        )
        .bind("SPY")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after, before);
        assert_eq!(after.0, CURRENT_FORMAT_VERSION);
    }

    #[tokio::test]
    async fn finds_latest_snapshot_at_or_before_target() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let first = sample_snapshot();
        let mut second = first.clone();
        second.timestamp_utc += chrono::Duration::hours(1);
        second.chains[0].contratos[0].bid = 11.0;
        second.contratos[0].bid = 11.0;
        save_snapshot(&pool, &first, first.timestamp_utc)
            .await
            .unwrap();
        save_snapshot(&pool, &second, second.timestamp_utc)
            .await
            .unwrap();

        let target = first.timestamp_utc + chrono::Duration::minutes(30);
        let loaded = load_latest_at_or_before(&pool, "SPY", target)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded, first);
    }

    #[tokio::test]
    async fn rejects_unsupported_payload_versions() {
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();
        let snapshot = sample_snapshot();
        save_snapshot(&pool, &snapshot, snapshot.timestamp_utc)
            .await
            .unwrap();
        sqlx::query("UPDATE option_snapshots SET format_version = 99")
            .execute(&pool)
            .await
            .unwrap();

        let error = load_latest(&pool, "SPY").await.unwrap_err();

        assert_eq!(
            error.to_string(),
            SnapshotStorageError::UnsupportedFormat(99).to_string()
        );
    }

    #[tokio::test]
    async fn parses_stores_and_loads_complete_snapshot_fixture() {
        let json = include_str!("../../../tests/fixtures/snapshot.json");
        let response: crate::driven_adapters::cboe::CboeResponse =
            serde_json::from_str(json).expect("o fixture deve conter um DTO CBOE válido");
        let snapshot = crate::driven_adapters::cboe::response_to_snapshot("SPY", response)
            .expect("o DTO completo deve ser convertido para o domínio");
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();

        assert_eq!(snapshot.ticker, "SPY");
        assert!(snapshot.contratos.len() > 1_000);
        assert!(
            snapshot
                .contratos
                .iter()
                .any(|contract| contract.implied_volatility.is_some()),
            "o campo IV da CBOE deve chegar ao payload de domínio"
        );
        assert_eq!(
            snapshot.contratos.len(),
            snapshot
                .chains
                .iter()
                .map(|chain| chain.contratos.len())
                .sum::<usize>()
        );

        save_snapshot(&pool, &snapshot, snapshot.timestamp_utc)
            .await
            .unwrap();
        let payload_size: i64 =
            sqlx::query_scalar("SELECT length(payload) FROM option_snapshots WHERE ticker = ?")
                .bind("SPY")
                .fetch_one(&pool)
                .await
                .unwrap();
        let loaded = load_latest(&pool, "SPY").await.unwrap().unwrap();

        assert_eq!(loaded, snapshot);
        assert!((payload_size as usize) < json.len());
        println!(
            "{} contratos; JSON: {} bytes; MessagePack: {} bytes",
            loaded.contratos.len(),
            json.len(),
            payload_size
        );
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente da CBOE"]
    async fn downloads_parses_stores_and_loads_real_snapshot() {
        let response = crate::driven_adapters::cboe::download_snapshot("SPY")
            .await
            .expect("a CBOE deve devolver o snapshot de SPY");
        let snapshot = crate::driven_adapters::cboe::response_to_snapshot("SPY", response)
            .expect("o DTO CBOE deve ser convertido para o domínio");
        let pool = memory_pool().await;
        initialize(&pool).await.unwrap();

        save_snapshot(&pool, &snapshot, snapshot.timestamp_utc)
            .await
            .unwrap();
        let loaded = load_latest(&pool, "SPY").await.unwrap().unwrap();

        assert_eq!(loaded, snapshot);
    }
}
