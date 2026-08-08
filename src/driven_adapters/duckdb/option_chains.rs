//! Columnar DuckDB persistence for option-chain snapshots and contracts.

use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use duckdb::{Connection, OptionalExt, params};

use crate::hexagon::{
    PortError, PortResult,
    domain::options::{ContratoOpcao, OptionChain, OptionType, Snapshot},
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
            PRIMARY KEY (snapshot_id, occ_symbol)
        );",
    )?;
    remove_legacy_hash_column(connection)?;
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
         (snapshot_id, ticker, observed_at, market_close, format_version)
         VALUES (?, ?, ?, ?, 1)
         ON CONFLICT DO NOTHING",
        params![&snapshot_id, ticker, snapshot.timestamp_utc, market_close,],
    )?;
    if inserted == 0 {
        transaction.commit()?;
        return Ok(0);
    }

    {
        // DuckDB's Appender batches values into vectors instead of executing
        // one SQL statement per contract. This is essential for large chains.
        let mut appender = transaction.appender("option_contracts")?;
        for chain in &snapshot.chains {
            for contract in &chain.contratos {
                appender.append_row(params![
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
                ])?;
            }
        }
        appender.flush()?;
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
            "SELECT snapshot_id, ticker, observed_at
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
                ))
            },
        )
        .optional()?;
    let Some((snapshot_id, ticker, timestamp_utc)) = metadata else {
        return Ok(None);
    };

    let mut statement = connection.prepare(
        "SELECT occ_symbol, root, expiration, option_type, strike, bid, ask,
                mid, spread, volume, open_interest, implied_volatility, delta,
                gamma, vega, theta, rho, theo
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
    }))
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
            open_interest: row.get(10)?,
            implied_volatility: row.get(11)?,
            delta: row.get(12)?,
            gamma: row.get(13)?,
            vega: row.get(14)?,
            theta: row.get(15)?,
            rho: row.get(16)?,
            theo: row.get(17)?,
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
