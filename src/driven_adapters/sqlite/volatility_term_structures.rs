use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Row, SqlitePool};
use std::error::Error;
use std::io;

use crate::hexagon::domain::volatility::{
    ConstantMaturityVolatilityPoint, TermStructure, TermStructurePoint, TermStructureSource,
};

pub const CALCULATION_VERSION: i64 = 3;

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    rename_table_if_needed(
        pool,
        "cboe_term_structure_points",
        "volatility_term_structure_points",
    )
    .await?;
    sqlx::query("DROP INDEX IF EXISTS idx_cboe_term_structure_points_snapshot")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS volatility_term_structure_points (
            ticker TEXT NOT NULL,
            snapshot_timestamp TIMESTAMP NOT NULL,
            treasury_date DATE NOT NULL,
            calculation_version INTEGER NOT NULL,
            days REAL NOT NULL,
            variance REAL NOT NULL,
            volatility REAL NOT NULL,
            source_type TEXT NOT NULL,
            expiration DATE,
            interest_rate REAL,
            near_expiration DATE,
            near_rate REAL,
            next_expiration DATE,
            next_rate REAL,
            PRIMARY KEY (ticker, snapshot_timestamp, calculation_version, days)
        )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_volatility_term_structure_points_snapshot
         ON volatility_term_structure_points (snapshot_timestamp)",
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

pub async fn insert(pool: &SqlitePool, term_structure: &TermStructure) -> Result<u64, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let mut inserted = 0;
    for point in &term_structure.points {
        let (
            source_type,
            expiration,
            interest_rate,
            near_expiration,
            near_rate,
            next_expiration,
            next_rate,
        ) = match &point.source {
            TermStructureSource::Interpolated {
                near_expiration,
                near_rate,
                next_expiration,
                next_rate,
            } => (
                "interpolated",
                None,
                None,
                Some(*near_expiration),
                Some(*near_rate),
                Some(*next_expiration),
                Some(*next_rate),
            ),
            TermStructureSource::Expiration {
                expiration,
                interest_rate,
            } => (
                "expiration",
                Some(*expiration),
                Some(*interest_rate),
                None,
                None,
                None,
                None,
            ),
        };
        let result = sqlx::query(
            "INSERT INTO volatility_term_structure_points (
                ticker, snapshot_timestamp, treasury_date, calculation_version,
                days, variance, volatility, source_type, expiration, interest_rate,
                near_expiration, near_rate, next_expiration, next_rate
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (ticker, snapshot_timestamp, calculation_version, days) DO NOTHING",
        )
        .bind(term_structure.ticker.trim().to_ascii_uppercase())
        .bind(term_structure.snapshot_timestamp)
        .bind(term_structure.treasury_date)
        .bind(CALCULATION_VERSION)
        .bind(point.days)
        .bind(point.variance)
        .bind(point.volatility)
        .bind(source_type)
        .bind(expiration)
        .bind(interest_rate)
        .bind(near_expiration)
        .bind(near_rate)
        .bind(next_expiration)
        .bind(next_rate)
        .execute(&mut *transaction)
        .await?;
        inserted += result.rows_affected();
    }
    transaction.commit().await?;
    Ok(inserted)
}

pub async fn load(
    pool: &SqlitePool,
    ticker: &str,
    snapshot_timestamp: DateTime<Utc>,
) -> Result<Option<TermStructure>, Box<dyn Error + Send + Sync>> {
    let ticker = ticker.trim().to_ascii_uppercase();
    let rows = sqlx::query(
        "SELECT treasury_date, days, variance, volatility, source_type,
                expiration, interest_rate, near_expiration, near_rate,
                next_expiration, next_rate
         FROM volatility_term_structure_points
         WHERE ticker = ? AND snapshot_timestamp = ? AND calculation_version = ?
         ORDER BY days",
    )
    .bind(&ticker)
    .bind(snapshot_timestamp)
    .bind(CALCULATION_VERSION)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }

    let treasury_date: NaiveDate = rows[0].try_get("treasury_date")?;
    let points = rows
        .into_iter()
        .map(row_to_point)
        .collect::<Result<Vec<_>, Box<dyn Error + Send + Sync>>>()?;
    Ok(Some(TermStructure {
        ticker,
        snapshot_timestamp,
        treasury_date,
        points,
    }))
}

pub async fn load_constant_maturity_history(
    pool: &SqlitePool,
    ticker: &str,
    target_days: f64,
) -> Result<Vec<ConstantMaturityVolatilityPoint>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT snapshot_timestamp, volatility
         FROM volatility_term_structure_points
         WHERE ticker = ? AND calculation_version = ? AND ABS(days - ?) < 0.000001
         ORDER BY snapshot_timestamp",
    )
    .bind(ticker.trim().to_ascii_uppercase())
    .bind(CALCULATION_VERSION)
    .bind(target_days)
    .fetch_all(pool)
    .await?;
    let mut by_session = std::collections::BTreeMap::new();
    for row in rows {
        let timestamp: DateTime<Utc> = row.try_get("snapshot_timestamp")?;
        by_session.insert(timestamp.date_naive(), row.try_get("volatility")?);
    }
    Ok(by_session
        .into_iter()
        .map(|(date, volatility)| ConstantMaturityVolatilityPoint {
            date,
            target_days,
            volatility,
        })
        .collect())
}

fn row_to_point(
    row: sqlx::sqlite::SqliteRow,
) -> Result<TermStructurePoint, Box<dyn Error + Send + Sync>> {
    let source_type: String = row.try_get("source_type")?;
    let source = match source_type.as_str() {
        "interpolated" => TermStructureSource::Interpolated {
            near_expiration: required(&row, "near_expiration")?,
            near_rate: required(&row, "near_rate")?,
            next_expiration: required(&row, "next_expiration")?,
            next_rate: required(&row, "next_rate")?,
        },
        "expiration" => TermStructureSource::Expiration {
            expiration: required(&row, "expiration")?,
            interest_rate: required(&row, "interest_rate")?,
        },
        _ => return Err(invalid_data("tipo de ponto desconhecido")),
    };
    Ok(TermStructurePoint {
        days: row.try_get("days")?,
        variance: row.try_get("variance")?,
        volatility: row.try_get("volatility")?,
        source,
    })
}

fn required<T>(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<T, sqlx::Error>
where
    T: for<'row> sqlx::Decode<'row, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    row.try_get::<Option<T>, _>(column)?
        .ok_or_else(|| sqlx::Error::ColumnDecode {
            index: column.to_string(),
            source: Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("campo obrigatório ausente: {column}"),
            )),
        })
}

fn invalid_data(message: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn stores_and_loads_variable_term_structure_points_idempotently() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        initialize(&pool).await.unwrap();
        let structure = TermStructure {
            ticker: "SPY".to_string(),
            snapshot_timestamp: Utc.with_ymd_and_hms(2026, 7, 14, 9, 41, 15).unwrap(),
            treasury_date: NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
            points: vec![
                TermStructurePoint {
                    days: 30.0,
                    variance: 0.03,
                    volatility: 17.32,
                    source: TermStructureSource::Interpolated {
                        near_expiration: NaiveDate::from_ymd_opt(2026, 8, 7).unwrap(),
                        near_rate: 0.036,
                        next_expiration: NaiveDate::from_ymd_opt(2026, 8, 14).unwrap(),
                        next_rate: 0.037,
                    },
                },
                TermStructurePoint {
                    days: 32.0,
                    variance: 0.031,
                    volatility: 17.61,
                    source: TermStructureSource::Expiration {
                        expiration: NaiveDate::from_ymd_opt(2026, 8, 14).unwrap(),
                        interest_rate: 0.037,
                    },
                },
            ],
        };

        assert_eq!(insert(&pool, &structure).await.unwrap(), 2);
        assert_eq!(insert(&pool, &structure).await.unwrap(), 0);
        assert_eq!(
            load(&pool, "spy", structure.snapshot_timestamp)
                .await
                .unwrap(),
            Some(structure)
        );
    }

    #[tokio::test]
    async fn loads_one_latest_30_day_point_per_market_session() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        initialize(&pool).await.unwrap();
        for (timestamp, volatility) in [
            ("2026-07-14T10:00:00Z", 24.0),
            ("2026-07-14T21:00:00Z", 25.0),
            ("2026-07-15T21:00:00Z", 26.0),
        ] {
            sqlx::query(
                "INSERT INTO volatility_term_structure_points (
                    ticker, snapshot_timestamp, treasury_date, calculation_version,
                    days, variance, volatility, source_type
                 ) VALUES ('IBM', ?, '2026-07-13', ?, 30.0, 0.04, ?, 'interpolated')",
            )
            .bind(timestamp)
            .bind(CALCULATION_VERSION)
            .bind(volatility)
            .execute(&pool)
            .await
            .unwrap();
        }

        let points = load_constant_maturity_history(&pool, "ibm", 30.0)
            .await
            .unwrap();

        assert_eq!(points.len(), 2);
        assert_eq!(
            points[0].date,
            NaiveDate::from_ymd_opt(2026, 7, 14).unwrap()
        );
        assert_eq!(points[0].volatility, 25.0);
        assert_eq!(points[1].volatility, 26.0);
    }

    #[tokio::test]
    async fn migrates_legacy_term_structure_table_without_losing_rows() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE cboe_term_structure_points (
                ticker TEXT NOT NULL, snapshot_timestamp TIMESTAMP NOT NULL,
                treasury_date DATE NOT NULL, calculation_version INTEGER NOT NULL,
                days REAL NOT NULL, variance REAL NOT NULL, volatility REAL NOT NULL,
                source_type TEXT NOT NULL, expiration DATE, interest_rate REAL,
                near_expiration DATE, near_rate REAL, next_expiration DATE, next_rate REAL,
                PRIMARY KEY (ticker, snapshot_timestamp, calculation_version, days)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO cboe_term_structure_points (
                ticker, snapshot_timestamp, treasury_date, calculation_version,
                days, variance, volatility, source_type
             ) VALUES ('SPY', '2026-07-14T20:00:00Z', '2026-07-14', 1,
                       30.0, 0.03, 17.32, 'expiration')",
        )
        .execute(&pool)
        .await
        .unwrap();

        initialize(&pool).await.unwrap();

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM volatility_term_structure_points")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 1);
    }
}
