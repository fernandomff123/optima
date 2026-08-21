//! Columnar DuckDB persistence for option-chain snapshots and contracts.

use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use duckdb::{Connection, OptionalExt, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::options::{
        ContratoOpcao, OptionChain, OptionContractSpecification, OptionIngestionDiagnostics,
        OptionIngestionWarning, OptionType, ProviderTimestamp, ProviderTimestampTimezone, Snapshot,
        UnderlyingPriceObservation,
    },
    driven_ports::{
        for_counting_option_chains::{ForCountingOptionChains, OptionChainCounts},
        for_loading_option_chains::ForLoadingOptionChains,
        for_storing_option_chains::ForStoringOptionChains,
    },
};

#[derive(Debug, Clone)]
pub struct DuckDbOptionChainsAdapter {
    database_path: PathBuf,
}

#[async_trait::async_trait]
impl ForCountingOptionChains for DuckDbOptionChainsAdapter {
    async fn count_option_chains(&self) -> PortResult<OptionChainCounts> {
        let (snapshots, contracts) = self.counts().await?;
        Ok(OptionChainCounts {
            snapshots,
            contracts,
        })
    }
}

impl DuckDbOptionChainsAdapter {
    pub fn new(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
        }
    }

    /// Creates only the schema owned by this adapter.
    pub async fn initialize(&self) -> PortResult<()> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)
        })
        .await
    }

    /// Returns physical row counts for migration verification and diagnostics.
    pub async fn counts(&self) -> PortResult<(u64, u64)> {
        let path = self.database_path.clone();
        run_blocking(move || {
            let connection = Connection::open(path)?;
            initialize_schema(&connection)?;
            connection.query_row(
                "SELECT
                    (SELECT COUNT(*) FROM option_snapshots),
                    (SELECT COUNT(*) FROM option_contracts)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
        })
        .await
    }
}

#[async_trait::async_trait]
impl ForStoringOptionChains for DuckDbOptionChainsAdapter {
    async fn store_option_chain(
        &self,
        snapshot: &Snapshot,
        market_close: DateTime<Utc>,
    ) -> PortResult<u64> {
        let path = self.database_path.clone();
        let snapshot = snapshot.clone();
        run_blocking(move || store_snapshot(&path, &snapshot, market_close)).await
    }
}

#[async_trait::async_trait]
impl ForLoadingOptionChains for DuckDbOptionChainsAdapter {
    async fn load_option_chain(&self, ticker: &str) -> PortResult<Option<Snapshot>> {
        let path = self.database_path.clone();
        let ticker = ticker.trim().to_ascii_uppercase();
        run_blocking(move || load_latest_snapshot(&path, &ticker)).await
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), duckdb::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS option_snapshots (
            snapshot_id VARCHAR PRIMARY KEY,
            ticker VARCHAR NOT NULL,
            observed_at TIMESTAMPTZ NOT NULL,
            market_close TIMESTAMPTZ,
            format_version INTEGER NOT NULL,
            UNIQUE (ticker, market_close)
        );
        CREATE TABLE IF NOT EXISTS option_contracts (
            snapshot_id VARCHAR NOT NULL,
            occ_symbol VARCHAR NOT NULL,
            root VARCHAR NOT NULL,
            expiration DATE NOT NULL,
            option_type VARCHAR NOT NULL,
            strike DOUBLE NOT NULL,
            bid DOUBLE,
            ask DOUBLE,
            mid DOUBLE,
            spread DOUBLE,
            volume DOUBLE,
            open_interest DOUBLE,
            implied_volatility DOUBLE,
            delta DOUBLE,
            gamma DOUBLE,
            vega DOUBLE,
            theta DOUBLE,
            rho DOUBLE,
            theo DOUBLE,
            contract_multiplier DOUBLE,
            contract_currency VARCHAR,
            contract_specification_source VARCHAR,
            contract_specification_reviewed_at DATE,
            contract_specification_effective_from DATE,
            PRIMARY KEY (snapshot_id, occ_symbol)
        );",
    )?;
    remove_legacy_hash_column(connection)?;
    connection.execute_batch(
        "ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS spot DOUBLE;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS spot_as_of TIMESTAMPTZ;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS collected_at TIMESTAMPTZ;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS spot_currency VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS spot_source VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS spot_as_of_raw VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS spot_as_of_timezone VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS provider_timestamp_raw VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS provider_timestamp_timezone VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS invalid_occ_symbols VARCHAR;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS invalid_occ_symbol_count UBIGINT;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS ingestion_warning_count UBIGINT;
         ALTER TABLE option_snapshots ADD COLUMN IF NOT EXISTS ingestion_warnings VARCHAR;
         ALTER TABLE option_contracts ADD COLUMN IF NOT EXISTS contract_multiplier DOUBLE;
         ALTER TABLE option_contracts ADD COLUMN IF NOT EXISTS contract_currency VARCHAR;
         ALTER TABLE option_contracts ADD COLUMN IF NOT EXISTS contract_specification_source VARCHAR;
         ALTER TABLE option_contracts ADD COLUMN IF NOT EXISTS contract_specification_reviewed_at DATE;
         ALTER TABLE option_contracts ADD COLUMN IF NOT EXISTS contract_specification_effective_from DATE;",
    )?;
    connection.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_option_snapshots_ticker_time
            ON option_snapshots (ticker, observed_at);
        CREATE INDEX IF NOT EXISTS idx_option_contracts_root_expiration
            ON option_contracts (root, expiration);
        CREATE INDEX IF NOT EXISTS idx_option_contracts_snapshot
            ON option_contracts (snapshot_id);",
    )
}

/// Rebuilds only the small snapshot metadata table. Contract rows keep their
/// existing opaque snapshot identifiers and therefore require no rewrite.
fn remove_legacy_hash_column(connection: &Connection) -> Result<(), duckdb::Error> {
    let has_hash: bool = connection.query_row(
        "SELECT COUNT(*) > 0
         FROM information_schema.columns
         WHERE table_name = 'option_snapshots'
           AND column_name = 'payload_hash'",
        [],
        |row| row.get(0),
    )?;
    if !has_hash {
        return Ok(());
    }

    connection.execute_batch(
        "BEGIN;
         CREATE TABLE option_snapshots_without_hash (
             snapshot_id VARCHAR PRIMARY KEY,
             ticker VARCHAR NOT NULL,
             observed_at TIMESTAMPTZ NOT NULL,
             market_close TIMESTAMPTZ,
             format_version INTEGER NOT NULL,
             UNIQUE (ticker, market_close)
         );
         INSERT INTO option_snapshots_without_hash
             (snapshot_id, ticker, observed_at, market_close, format_version)
         SELECT snapshot_id, ticker, observed_at, market_close, format_version
         FROM option_snapshots;
         DROP TABLE option_snapshots;
         ALTER TABLE option_snapshots_without_hash RENAME TO option_snapshots;
         COMMIT;",
    )
}

fn store_snapshot(
    path: &PathBuf,
    snapshot: &Snapshot,
    market_close: DateTime<Utc>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    if snapshot.chains.is_empty() {
        return Err("cannot store an option snapshot without chains".into());
    }
    let mut connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let ticker = snapshot.ticker.trim().to_ascii_uppercase();
    let snapshot_id = format!("{ticker}@{}", market_close.to_rfc3339());
    let transaction = connection.transaction()?;
    let inserted = transaction.execute(
        "INSERT INTO option_snapshots
         (snapshot_id, ticker, observed_at, market_close, format_version,
          spot, spot_as_of, collected_at, spot_currency, spot_source,
          spot_as_of_raw, spot_as_of_timezone, provider_timestamp_raw, provider_timestamp_timezone,
          invalid_occ_symbol_count, invalid_occ_symbols,
          ingestion_warning_count, ingestion_warnings)
         VALUES (?, ?, ?, ?, 2, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT DO NOTHING",
        params![
            &snapshot_id,
            ticker,
            snapshot.timestamp_utc,
            market_close,
            snapshot.underlying_price.as_ref().map(|spot| spot.value),
            snapshot
                .underlying_price
                .as_ref()
                .and_then(|spot| spot.observed_at),
            snapshot.collected_at,
            snapshot
                .underlying_price
                .as_ref()
                .and_then(|spot| spot.currency.as_deref()),
            snapshot
                .underlying_price
                .as_ref()
                .map(|spot| spot.source.as_str()),
            snapshot
                .underlying_price
                .as_ref()
                .and_then(|spot| spot.observed_at_raw.as_deref()),
            snapshot
                .underlying_price
                .as_ref()
                .and_then(|spot| spot.observed_at_timezone.as_ref())
                .map(timestamp_timezone_value),
            snapshot
                .provider_timestamp
                .as_ref()
                .map(|timestamp| timestamp.raw.as_str()),
            snapshot
                .provider_timestamp
                .as_ref()
                .map(|timestamp| { timestamp_timezone_value(&timestamp.timezone) }),
            snapshot.ingestion_diagnostics.invalid_occ_symbol_count,
            serde_json::to_string(&snapshot.ingestion_diagnostics.invalid_occ_symbol_samples)?,
            snapshot.ingestion_diagnostics.warning_count,
            serde_json::to_string(&snapshot.ingestion_diagnostics.warnings)?,
        ],
    )?;
    if inserted == 0 {
        transaction.commit()?;
        return Ok(0);
    }

    {
        // Explicit columns make schema evolution independent from physical
        // column order in databases migrated through different versions.
        let mut statement = transaction.prepare(
            "INSERT INTO option_contracts
             (snapshot_id, occ_symbol, root, expiration, option_type, strike,
              bid, ask, mid, spread, volume, open_interest, implied_volatility,
              delta, gamma, vega, theta, rho, theo, contract_multiplier,
              contract_currency, contract_specification_source,
              contract_specification_reviewed_at,
              contract_specification_effective_from)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        for chain in &snapshot.chains {
            for contract in &chain.contratos {
                statement.execute(params![
                    &snapshot_id,
                    &contract.occ_symbol,
                    &chain.root,
                    contract.expiration,
                    contract.option_type.to_string(),
                    contract.strike,
                    contract.bid,
                    contract.ask,
                    contract.mid,
                    contract.spread,
                    contract.volume,
                    contract.open_interest,
                    contract.implied_volatility,
                    contract.delta,
                    contract.gamma,
                    contract.vega,
                    contract.theta,
                    contract.rho,
                    contract.theo,
                    contract
                        .contract_specification
                        .as_ref()
                        .map(|specification| specification.contract_multiplier),
                    contract
                        .contract_specification
                        .as_ref()
                        .map(|specification| specification.currency.as_str()),
                    contract
                        .contract_specification
                        .as_ref()
                        .map(|specification| specification.source_reference.as_str()),
                    contract
                        .contract_specification
                        .as_ref()
                        .and_then(|specification| specification.catalog_reviewed_at),
                    contract
                        .contract_specification
                        .as_ref()
                        .and_then(|specification| specification.effective_from),
                ])?;
            }
        }
    }
    transaction.commit()?;
    Ok(1)
}

fn load_latest_snapshot(
    path: &PathBuf,
    ticker: &str,
) -> Result<Option<Snapshot>, Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::open(path)?;
    initialize_schema(&connection)?;
    let metadata = connection
        .query_row(
            "SELECT snapshot_id, ticker, observed_at, spot, spot_as_of,
                    collected_at, spot_currency, spot_source,
                    spot_as_of_raw, spot_as_of_timezone,
                    provider_timestamp_raw, provider_timestamp_timezone,
                    invalid_occ_symbol_count, invalid_occ_symbols,
                    ingestion_warning_count, ingestion_warnings
             FROM option_snapshots
             WHERE ticker = ?
             ORDER BY observed_at DESC
             LIMIT 1",
            [ticker],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, DateTime<Utc>>(2)?,
                    row.get::<_, Option<f64>>(3)?,
                    row.get::<_, Option<DateTime<Utc>>>(4)?,
                    row.get::<_, Option<DateTime<Utc>>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<u64>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<u64>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                ))
            },
        )
        .optional()?;
    let Some((
        snapshot_id,
        ticker,
        timestamp_utc,
        spot,
        spot_as_of,
        collected_at,
        spot_currency,
        spot_source,
        spot_as_of_raw,
        spot_as_of_timezone,
        provider_timestamp_raw,
        provider_timestamp_timezone,
        invalid_occ_symbol_count,
        invalid_occ_symbols,
        ingestion_warning_count,
        ingestion_warnings,
    )) = metadata
    else {
        return Ok(None);
    };

    let mut statement = connection.prepare(
        "SELECT occ_symbol, root, expiration, option_type, strike, bid, ask,
                mid, spread, volume, open_interest, implied_volatility, delta,
                gamma, vega, theta, rho, theo, contract_multiplier,
                contract_currency, contract_specification_source,
                contract_specification_reviewed_at,
                contract_specification_effective_from
         FROM option_contracts
         WHERE snapshot_id = ?
         ORDER BY root, expiration, option_type, strike, occ_symbol",
    )?;
    let rows = statement.query_map([&snapshot_id], contract_from_row)?;
    let mut by_root = BTreeMap::<String, Vec<ContratoOpcao>>::new();
    for row in rows {
        let (root, contract) = row?;
        by_root.entry(root).or_default().push(contract);
    }
    let chains = by_root
        .into_iter()
        .map(|(root, contratos)| OptionChain { root, contratos })
        .collect::<Vec<_>>();
    let contratos = chains
        .iter()
        .flat_map(|chain| chain.contratos.iter().cloned())
        .collect();
    Ok(Some(Snapshot {
        ticker,
        timestamp_utc,
        contratos,
        chains,
        underlying_price: spot.and_then(|value| {
            UnderlyingPriceObservation::new(
                value,
                spot_as_of,
                spot_currency,
                spot_source.unwrap_or_default(),
            )
            .map(|observation| {
                observation.with_provider_timestamp(
                    spot_as_of_raw,
                    spot_as_of_timezone.as_deref().map(parse_timestamp_timezone),
                )
            })
        }),
        collected_at,
        provider_timestamp: provider_timestamp_raw.map(|raw| ProviderTimestamp {
            raw,
            timezone: parse_timestamp_timezone(
                provider_timestamp_timezone
                    .as_deref()
                    .unwrap_or("unverified"),
            ),
        }),
        ingestion_diagnostics: {
            let invalid_occ_symbol_samples: Vec<String> = invalid_occ_symbols
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default();
            let warnings: Vec<OptionIngestionWarning> = ingestion_warnings
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default();
            OptionIngestionDiagnostics {
                invalid_occ_symbol_count: invalid_occ_symbol_count
                    .unwrap_or(invalid_occ_symbol_samples.len() as u64),
                invalid_occ_symbol_samples,
                warning_count: ingestion_warning_count.unwrap_or(warnings.len() as u64),
                warnings,
            }
        },
    }))
}

fn timestamp_timezone_value(timezone: &ProviderTimestampTimezone) -> &'static str {
    match timezone {
        ProviderTimestampTimezone::VerifiedOffset => "verified_offset",
        ProviderTimestampTimezone::Unverified => "unverified",
    }
}

fn parse_timestamp_timezone(value: &str) -> ProviderTimestampTimezone {
    match value {
        "verified_offset" => ProviderTimestampTimezone::VerifiedOffset,
        _ => ProviderTimestampTimezone::Unverified,
    }
}

fn contract_from_row(row: &duckdb::Row<'_>) -> Result<(String, ContratoOpcao), duckdb::Error> {
    let option_type = match row.get::<_, String>(3)?.as_str() {
        "Call" => OptionType::Call,
        "Put" => OptionType::Put,
        value => {
            return Err(duckdb::Error::FromSqlConversionFailure(
                3,
                duckdb::types::Type::Text,
                format!("invalid stored option type: {value}").into(),
            ));
        }
    };
    Ok((
        row.get(1)?,
        ContratoOpcao {
            occ_symbol: row.get(0)?,
            expiration: row.get::<_, NaiveDate>(2)?,
            option_type,
            strike: row.get(4)?,
            bid: row.get(5)?,
            ask: row.get(6)?,
            mid: row.get(7)?,
            spread: row.get(8)?,
            volume: row.get(9)?,
            open_interest: row.get::<_, Option<f64>>(10)?,
            implied_volatility: row.get(11)?,
            delta: row.get(12)?,
            gamma: row.get::<_, Option<f64>>(13)?,
            vega: row.get(14)?,
            theta: row.get(15)?,
            rho: row.get(16)?,
            theo: row.get(17)?,
            contract_specification: row.get::<_, Option<f64>>(18)?.and_then(|multiplier| {
                OptionContractSpecification::new(
                    row.get::<_, String>(1).ok()?,
                    multiplier,
                    row.get::<_, Option<String>>(19).ok()??,
                    row.get::<_, Option<String>>(20).ok()??,
                    row.get::<_, Option<NaiveDate>>(21).ok()?,
                    row.get::<_, Option<NaiveDate>>(22).ok()?,
                )
            }),
        },
    ))
}

async fn run_blocking<T, E>(
    operation: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> PortResult<T>
where
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| PortError::Unavailable(error.to_string()))?
        .map_err(|error| PortError::Unavailable(error.to_string()))
}
