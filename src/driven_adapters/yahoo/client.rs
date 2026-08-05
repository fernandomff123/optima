//! HTTP client and wire DTOs for Yahoo Finance.

#[cfg(test)]
use chrono::Days;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;

const YAHOO_CHART_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";

#[derive(Debug, Deserialize)]
pub struct YahooResponse {
    pub chart: YahooChart,
}

#[derive(Debug, Deserialize)]
pub struct YahooChart {
    pub result: Option<Vec<YahooChartResult>>,
    pub error: Option<YahooApiError>,
}

#[derive(Debug, Deserialize)]
pub struct YahooApiError {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct YahooChartResult {
    pub meta: YahooMeta,
    #[serde(default)]
    pub timestamp: Vec<i64>,
    pub indicators: YahooIndicators,
    pub events: Option<YahooEvents>,
}

#[derive(Debug, Deserialize)]
pub struct YahooMeta {
    pub symbol: String,
    pub currency: Option<String>,
    #[serde(rename = "exchangeName")]
    pub exchange_name: Option<String>,
    #[serde(rename = "exchangeTimezoneName")]
    pub exchange_timezone_name: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    pub regular_market_price: Option<f64>,
    #[serde(rename = "chartPreviousClose")]
    pub chart_previous_close: Option<f64>,
    #[serde(rename = "regularMarketTime")]
    pub regular_market_time: Option<i64>,
    #[serde(rename = "regularMarketVolume")]
    pub regular_market_volume: Option<i64>,
    /// Provider-reported session state, for example `REGULAR` or `CLOSED`.
    #[serde(rename = "marketState")]
    pub market_state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct YahooIndicators {
    #[serde(default)]
    pub quote: Vec<YahooQuote>,
    #[serde(default)]
    pub adjclose: Vec<YahooAdjustedClose>,
}

#[derive(Debug, Deserialize)]
pub struct YahooQuote {
    #[serde(default)]
    pub open: Vec<Option<f64>>,
    #[serde(default)]
    pub high: Vec<Option<f64>>,
    #[serde(default)]
    pub low: Vec<Option<f64>>,
    #[serde(default)]
    pub close: Vec<Option<f64>>,
    #[serde(default)]
    pub volume: Vec<Option<u64>>,
}

#[derive(Debug, Deserialize)]
pub struct YahooAdjustedClose {
    #[serde(default)]
    pub adjclose: Vec<Option<f64>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct YahooEvents {
    #[serde(default)]
    pub dividends: HashMap<String, YahooDividend>,
    #[serde(default)]
    pub splits: HashMap<String, YahooSplit>,
}

#[derive(Debug, Deserialize)]
pub struct YahooDividend {
    pub amount: f64,
    pub date: i64,
}

#[derive(Debug, Deserialize)]
pub struct YahooSplit {
    pub date: i64,
    pub numerator: f64,
    pub denominator: f64,
    #[serde(rename = "splitRatio")]
    pub split_ratio: String,
}

#[derive(Debug, PartialEq)]
pub enum YahooError {
    Ticker,
    StartDate,
    #[cfg(test)]
    EndDate,
    #[cfg(test)]
    DateRange,
}

impl fmt::Display for YahooError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ticker => write!(formatter, "ticker inválido"),
            Self::StartDate => write!(formatter, "a data inicial não pode estar no futuro"),
            #[cfg(test)]
            Self::EndDate => write!(formatter, "a data final não pode estar no futuro"),
            #[cfg(test)]
            Self::DateRange => {
                write!(
                    formatter,
                    "a data inicial não pode ser posterior à data final"
                )
            }
        }
    }
}

impl Error for YahooError {}

pub async fn download_historico(
    ticker: &str,
    data_inicio: NaiveDate,
) -> Result<YahooResponse, Box<dyn Error + Send + Sync>> {
    download_atualizacao(ticker, data_inicio).await
}

#[cfg(test)]
pub async fn download_inicial(
    ticker: &str,
    data_fim: NaiveDate,
) -> Result<YahooResponse, Box<dyn Error + Send + Sync>> {
    validate_end_date(data_fim)?;
    download_periodo(ticker, 0, end_exclusive_timestamp(data_fim)?).await
}

pub async fn download_atualizacao(
    ticker: &str,
    data_inicio: NaiveDate,
) -> Result<YahooResponse, Box<dyn Error + Send + Sync>> {
    let agora = Utc::now();
    if data_inicio > agora.date_naive() {
        return Err(YahooError::StartDate.into());
    }

    download_periodo(ticker, start_timestamp(data_inicio)?, agora.timestamp()).await
}

#[cfg(test)]
pub async fn download_intervalo(
    ticker: &str,
    data_inicio: NaiveDate,
    data_fim: NaiveDate,
) -> Result<YahooResponse, Box<dyn Error + Send + Sync>> {
    if data_inicio > data_fim {
        return Err(YahooError::DateRange.into());
    }
    if data_inicio > Utc::now().date_naive() {
        return Err(YahooError::StartDate.into());
    }
    validate_end_date(data_fim)?;

    download_periodo(
        ticker,
        start_timestamp(data_inicio)?,
        end_exclusive_timestamp(data_fim)?,
    )
    .await
}

async fn download_periodo(
    ticker: &str,
    period1: i64,
    period2: i64,
) -> Result<YahooResponse, Box<dyn Error + Send + Sync>> {
    let ticker = normalize_ticker(ticker)?;
    let url = format!("{YAHOO_CHART_URL}/{ticker}");
    let cliente = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let resposta = cliente
        .get(url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .query(&[
            ("period1", period1.to_string()),
            ("period2", period2.to_string()),
            ("interval", "1d".to_string()),
            ("events", "div,splits".to_string()),
            ("includeAdjustedClose", "true".to_string()),
        ])
        .send()
        .await?
        .error_for_status()?;

    Ok(resposta.json().await?)
}

#[cfg(test)]
fn validate_end_date(data_fim: NaiveDate) -> Result<(), YahooError> {
    if data_fim > Utc::now().date_naive() {
        return Err(YahooError::EndDate);
    }
    Ok(())
}

fn start_timestamp(date: NaiveDate) -> Result<i64, YahooError> {
    date.and_hms_opt(0, 0, 0)
        .map(|value| value.and_utc().timestamp())
        .ok_or(YahooError::StartDate)
}

#[cfg(test)]
fn end_exclusive_timestamp(date: NaiveDate) -> Result<i64, YahooError> {
    date.checked_add_days(Days::new(1))
        .and_then(|value| value.and_hms_opt(0, 0, 0))
        .map(|value| value.and_utc().timestamp())
        .ok_or(YahooError::EndDate)
}

pub(crate) fn normalize_ticker(ticker: &str) -> Result<String, YahooError> {
    let ticker = ticker.trim();
    let valid = !ticker.is_empty()
        && ticker.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '^' | '=' | '_')
        });

    if !valid {
        return Err(YahooError::Ticker);
    }

    Ok(ticker.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_yahoo_dto() {
        let json = r#"
        {
          "chart": {
            "result": [{
              "meta": {
                "symbol": "AAPL",
                "currency": "USD",
                "exchangeName": "NMS",
                "instrumentType": "EQUITY",
                "exchangeTimezoneName": "America/New_York",
                "marketState": "REGULAR"
              },
              "timestamp": [1704205800, 1704292200],
              "indicators": {
                "quote": [{
                  "open": [185.0, null],
                  "high": [186.0, 188.0],
                  "low": [184.0, 185.0],
                  "close": [185.5, 187.0],
                  "volume": [1000, 1200]
                }],
                "adjclose": [{"adjclose": [184.9, 186.4]}]
              },
              "events": {
                "dividends": {
                  "1704205800": {"amount": 0.24, "date": 1704205800}
                },
                "splits": {
                  "1704292200": {
                    "date": 1704292200,
                    "numerator": 4.0,
                    "denominator": 1.0,
                    "splitRatio": "4:1"
                  }
                }
              }
            }],
            "error": null
          }
        }
        "#;

        let response: YahooResponse = serde_json::from_str(json).unwrap();
        let result = &response.chart.result.unwrap()[0];

        assert_eq!(result.meta.symbol, "AAPL");
        assert_eq!(result.meta.market_state.as_deref(), Some("REGULAR"));
        assert_eq!(result.timestamp, [1704205800, 1704292200]);
        assert_eq!(result.indicators.quote[0].open, [Some(185.0), None]);
        assert_eq!(result.indicators.adjclose[0].adjclose[1], Some(186.4));
        assert_eq!(result.events.as_ref().unwrap().dividends.len(), 1);
        assert_eq!(
            result.events.as_ref().unwrap().splits["1704292200"].split_ratio,
            "4:1"
        );
    }

    #[test]
    fn rejects_invalid_tickers() {
        assert_eq!(normalize_ticker(""), Err(YahooError::Ticker));
        assert_eq!(normalize_ticker("../AAPL"), Err(YahooError::Ticker));
        assert_eq!(normalize_ticker(" brk-b ").unwrap(), "BRK-B");
    }

    #[test]
    fn makes_end_date_inclusive_for_yahoo() {
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        assert_eq!(end_exclusive_timestamp(end_date).unwrap(), 1_738_368_000);
    }

    #[tokio::test]
    async fn rejects_invalid_date_ranges_before_downloading() {
        let start_date = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();

        let error = download_intervalo("AAPL", start_date, end_date)
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), YahooError::DateRange.to_string());
    }

    #[test]
    fn deserializes_api_errors_without_processing_them() {
        let response: YahooResponse = serde_json::from_str(
            r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found"}}}"#,
        )
        .unwrap();

        let error = response.chart.error.unwrap();
        assert_eq!(error.code, "Not Found");
        assert_eq!(error.description, "No data found");
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente da Yahoo Finance"]
    async fn downloads_initial_history_until_date() {
        let data_fim = NaiveDate::from_ymd_opt(1981, 1, 31).unwrap();
        let period2 = end_exclusive_timestamp(data_fim).unwrap();

        let response = download_inicial("AAPL", data_fim)
            .await
            .expect("a Yahoo deve devolver o histórico inicial");
        let result = response
            .chart
            .result
            .and_then(|results| results.into_iter().next())
            .expect("a resposta deve conter o histórico inicial de AAPL");

        assert!(!result.timestamp.is_empty());
        assert!(
            result
                .timestamp
                .iter()
                .all(|timestamp| *timestamp >= 0 && *timestamp < period2)
        );
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente da Yahoo Finance"]
    async fn downloads_update_from_date_until_now() {
        let data_inicio = Utc::now().date_naive() - chrono::Duration::days(14);
        let period1 = start_timestamp(data_inicio).unwrap();

        let response = download_atualizacao("AAPL", data_inicio)
            .await
            .expect("a Yahoo deve devolver uma resposta válida");

        assert!(response.chart.error.is_none());

        let result = response
            .chart
            .result
            .and_then(|results| results.into_iter().next())
            .expect("a resposta deve conter dados para AAPL");
        assert_eq!(result.meta.symbol, "AAPL");
        assert!(!result.timestamp.is_empty());
        assert!(
            result
                .timestamp
                .iter()
                .all(|timestamp| *timestamp >= period1)
        );

        let quote = result
            .indicators
            .quote
            .first()
            .expect("a resposta deve conter cotações");
        let number_of_timestamps = result.timestamp.len();
        assert_eq!(quote.open.len(), number_of_timestamps);
        assert_eq!(quote.high.len(), number_of_timestamps);
        assert_eq!(quote.low.len(), number_of_timestamps);
        assert_eq!(quote.close.len(), number_of_timestamps);
        assert_eq!(quote.volume.len(), number_of_timestamps);

        let adjusted_close = result
            .indicators
            .adjclose
            .first()
            .expect("a resposta deve conter preços de fecho ajustados");
        assert_eq!(adjusted_close.adjclose.len(), number_of_timestamps);
    }
}
