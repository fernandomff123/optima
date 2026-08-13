use serde::{Deserialize, Serialize};

use super::sector_performance::{SECTOR_BENCHMARK_TICKER, SECTORS};

const SYSTEM_INDEX_TICKERS: [&str; 2] = ["SPX", "VIX"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedTickerSource {
    System,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedTicker {
    pub ticker: String,
    pub source: TrackedTickerSource,
    pub active: bool,
    pub historical_prices: bool,
    pub option_snapshots: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackedTickerConfiguration {
    pub active: bool,
    pub historical_prices: bool,
    pub option_snapshots: bool,
}

impl TrackedTicker {
    pub fn user(ticker: &str, configuration: TrackedTickerConfiguration) -> Result<Self, String> {
        Ok(Self {
            ticker: normalize_ticker(ticker)?,
            source: TrackedTickerSource::User,
            active: configuration.active,
            historical_prices: configuration.historical_prices,
            option_snapshots: configuration.option_snapshots,
        })
    }

    pub fn configuration(&self) -> TrackedTickerConfiguration {
        TrackedTickerConfiguration {
            active: self.active,
            historical_prices: self.historical_prices,
            option_snapshots: self.option_snapshots,
        }
    }
}

pub fn normalize_ticker(value: &str) -> Result<String, String> {
    let ticker = value.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || ticker.len() > 15
        || !ticker.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '^' | '.' | '-')
        })
    {
        return Err("invalid tracked ticker".to_string());
    }
    Ok(ticker)
}

pub fn system_tickers() -> Vec<TrackedTicker> {
    SYSTEM_INDEX_TICKERS
        .into_iter()
        .chain(std::iter::once(SECTOR_BENCHMARK_TICKER))
        .chain(SECTORS.into_iter().map(|sector| sector.etf))
        .map(|ticker| TrackedTicker {
            ticker: ticker.to_string(),
            source: TrackedTickerSource::System,
            active: true,
            historical_prices: ticker != "VIX",
            option_snapshots: matches!(ticker, "SPX" | "SPY"),
        })
        .collect()
}

pub fn is_system_ticker(ticker: &str) -> bool {
    SYSTEM_INDEX_TICKERS.contains(&ticker)
        || ticker == SECTOR_BENCHMARK_TICKER
        || SECTORS.iter().any(|sector| sector.etf == ticker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_supported_symbols() {
        assert_eq!(normalize_ticker(" brk.b ").unwrap(), "BRK.B");
        assert_eq!(normalize_ticker("^spx").unwrap(), "^SPX");
    }

    #[test]
    fn rejects_invalid_symbols() {
        for ticker in ["", " ", "SP Y", "SPY/US", "ABCDEFGHIJKLMNOP"] {
            assert_eq!(
                normalize_ticker(ticker),
                Err("invalid tracked ticker".into())
            );
        }
    }

    #[test]
    fn mandatory_catalog_contains_benchmark_volatility_and_sectors() {
        let tickers = system_tickers();
        assert_eq!(tickers.len(), 14);
        assert!(
            tickers
                .iter()
                .all(|ticker| ticker.source == TrackedTickerSource::System)
        );
        assert!(tickers.iter().all(|ticker| ticker.active));
        assert!(
            SECTORS
                .iter()
                .all(|sector| tickers.iter().any(|ticker| ticker.ticker == sector.etf))
        );
    }
}
