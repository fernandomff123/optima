//! HTTP client and CSV wire DTOs for Cboe volatility indices.

use std::error::Error;
use std::fmt;
use std::time::Duration;

const CBOE_INDICES_URL: &str = "https://cdn.cboe.com/api/global/us_indices/daily_prices";
const OHLC_HEADER: &str = "DATE,OPEN,HIGH,LOW,CLOSE";

#[derive(Debug)]
pub struct CboeIndexResponse {
    pub ticker: String,
    pub rows: Vec<CboeIndexRowRaw>,
}

#[derive(Debug)]
pub struct CboeIndexRowRaw {
    pub date: String,
    pub open: Option<String>,
    pub high: Option<String>,
    pub low: Option<String>,
    pub close: String,
}

#[derive(Debug, PartialEq)]
pub enum CboeIndexApiError {
    Ticker,
    Header(String),
    Row { line: usize, columns: usize },
}

impl fmt::Display for CboeIndexApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ticker => write!(formatter, "ticker de índice CBOE inválido"),
            Self::Header(header) => {
                write!(formatter, "cabeçalho CSV CBOE inválido: {header}")
            }
            Self::Row { line, columns } => write!(
                formatter,
                "linha {line} do CSV CBOE tem {columns} colunas; eram esperadas 2 ou 5"
            ),
        }
    }
}

impl Error for CboeIndexApiError {}

pub async fn download_indice(
    ticker: &str,
) -> Result<CboeIndexResponse, Box<dyn Error + Send + Sync>> {
    let ticker = normalize_ticker(ticker)?;
    let url = format!("{CBOE_INDICES_URL}/{ticker}_History.csv");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let csv = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(parse_csv(ticker, &csv)?)
}

fn normalize_ticker(ticker: &str) -> Result<String, CboeIndexApiError> {
    let ticker = ticker.trim();
    if ticker.is_empty()
        || !ticker
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err(CboeIndexApiError::Ticker);
    }

    Ok(ticker.to_ascii_uppercase())
}

fn parse_csv(ticker: String, csv: &str) -> Result<CboeIndexResponse, CboeIndexApiError> {
    let mut lines = csv.lines();
    let header = lines
        .next()
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .trim();
    let close_only_header = format!("DATE,{ticker}");
    if header != OHLC_HEADER && header != close_only_header {
        return Err(CboeIndexApiError::Header(header.to_string()));
    }

    let mut rows = Vec::new();
    for (index, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let columns = line.split(',').map(str::trim).collect::<Vec<_>>();
        if columns.len() != 2 && columns.len() != 5 {
            return Err(CboeIndexApiError::Row {
                line: index + 2,
                columns: columns.len(),
            });
        }

        let row = if columns.len() == 5 {
            CboeIndexRowRaw {
                date: columns[0].to_string(),
                open: Some(columns[1].to_string()),
                high: Some(columns[2].to_string()),
                low: Some(columns[3].to_string()),
                close: columns[4].to_string(),
            }
        } else {
            CboeIndexRowRaw {
                date: columns[0].to_string(),
                open: None,
                high: None,
                low: None,
                close: columns[1].to_string(),
            }
        };
        rows.push(row);
    }

    Ok(CboeIndexResponse { ticker, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cboe_csv_dto() {
        let csv = "DATE,OPEN,HIGH,LOW,CLOSE\r\n\
                   01/02/1990,17.240000,18.000000,16.500000,17.500000\r\n";

        let response = parse_csv("VIX".to_string(), csv).unwrap();

        assert_eq!(response.ticker, "VIX");
        assert_eq!(response.rows.len(), 1);
        assert_eq!(response.rows[0].date, "01/02/1990");
        assert_eq!(response.rows[0].close, "17.500000");
    }

    #[test]
    fn parses_close_only_cboe_csv_dto() {
        let csv = "DATE,VVIX\n03/06/2006,71.730000\n";

        let response = parse_csv("VVIX".to_string(), csv).unwrap();

        assert_eq!(response.rows.len(), 1);
        assert_eq!(response.rows[0].open, None);
        assert_eq!(response.rows[0].close, "71.730000");
    }

    #[test]
    fn rejects_invalid_csv_rows() {
        let error = parse_csv(
            "VIX".to_string(),
            "DATE,OPEN,HIGH,LOW,CLOSE\n01/02/1990,17.24,18.00",
        )
        .unwrap_err();

        assert_eq!(
            error,
            CboeIndexApiError::Row {
                line: 2,
                columns: 3,
            }
        );
    }

    #[test]
    fn rejects_invalid_tickers() {
        assert_eq!(normalize_ticker(""), Err(CboeIndexApiError::Ticker));
        assert_eq!(normalize_ticker("../VIX"), Err(CboeIndexApiError::Ticker));
        assert_eq!(normalize_ticker(" vix ").unwrap(), "VIX");
    }
}
