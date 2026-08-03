use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt;

use super::client::YahooResponse;
use crate::hexagon::domain::market_history::{DailyQuote, Dividend, MarketHistory, StockSplit};

#[derive(Debug, PartialEq)]
pub enum YahooParseError {
    Api { code: String, description: String },
    EmptyResponse,
    MissingQuotes,
    InvalidTimestamp(i64),
}

impl fmt::Display for YahooParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api { code, description } => {
                write!(formatter, "erro da API Yahoo ({code}): {description}")
            }
            Self::EmptyResponse => write!(formatter, "a API Yahoo não devolveu dados"),
            Self::MissingQuotes => write!(formatter, "a resposta Yahoo não contém cotações"),
            Self::InvalidTimestamp(timestamp) => {
                write!(formatter, "timestamp Yahoo inválido: {timestamp}")
            }
        }
    }
}

impl Error for YahooParseError {}

pub fn response_to_market_history(
    response: YahooResponse,
) -> Result<MarketHistory, YahooParseError> {
    if let Some(error) = response.chart.error {
        return Err(YahooParseError::Api {
            code: error.code,
            description: error.description,
        });
    }

    let result = response
        .chart
        .result
        .and_then(|results| results.into_iter().next())
        .ok_or(YahooParseError::EmptyResponse)?;
    let quote = result
        .indicators
        .quote
        .into_iter()
        .next()
        .ok_or(YahooParseError::MissingQuotes)?;
    let adjusted_close = result
        .indicators
        .adjclose
        .into_iter()
        .next()
        .map(|indicator| indicator.adjclose)
        .unwrap_or_default();

    let events = result.events.unwrap_or_default();
    let mut dividends = events
        .dividends
        .into_values()
        .map(|event| {
            Ok(Dividend {
                timestamp: parse_timestamp(event.date)?,
                amount: event.amount,
            })
        })
        .collect::<Result<Vec<_>, YahooParseError>>()?;
    let mut splits = events
        .splits
        .into_values()
        .map(|event| {
            Ok(StockSplit {
                timestamp: parse_timestamp(event.date)?,
                numerator: event.numerator,
                denominator: event.denominator,
                ratio: event.split_ratio,
            })
        })
        .collect::<Result<Vec<_>, YahooParseError>>()?;
    dividends.sort_by_key(|event| event.timestamp);
    splits.sort_by_key(|event| event.timestamp);

    let daily_quotes = result
        .timestamp
        .into_iter()
        .enumerate()
        .map(|(index, timestamp)| {
            Ok(DailyQuote {
                timestamp: parse_timestamp(timestamp)?,
                open: value_at(&quote.open, index),
                high: value_at(&quote.high, index),
                low: value_at(&quote.low, index),
                close: value_at(&quote.close, index),
                adjusted_close: value_at(&adjusted_close, index),
                volume: value_at(&quote.volume, index),
            })
        })
        .collect::<Result<Vec<_>, YahooParseError>>()?;

    Ok(MarketHistory {
        ticker: result.meta.symbol,
        currency: result.meta.currency,
        exchange_timezone: result.meta.exchange_timezone_name,
        daily_quotes,
        dividends,
        splits,
    })
}

fn parse_timestamp(timestamp: i64) -> Result<DateTime<Utc>, YahooParseError> {
    DateTime::from_timestamp(timestamp, 0).ok_or(YahooParseError::InvalidTimestamp(timestamp))
}

fn value_at<T: Copy>(values: &[Option<T>], index: usize) -> Option<T> {
    values.get(index).copied().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_quotes_and_events_to_domain() {
        let response: YahooResponse = serde_json::from_str(
            r#"
            {
              "chart": {
                "result": [{
                  "meta": {
                    "symbol": "AAPL",
                    "currency": "USD",
                    "exchangeTimezoneName": "America/New_York"
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
            "#,
        )
        .unwrap();

        let history = response_to_market_history(response).unwrap();

        assert_eq!(history.ticker, "AAPL");
        assert_eq!(history.currency.as_deref(), Some("USD"));
        assert_eq!(history.daily_quotes.len(), 2);
        assert_eq!(history.daily_quotes[0].open, Some(185.0));
        assert_eq!(history.daily_quotes[1].open, None);
        assert_eq!(history.dividends.len(), 1);
        assert_eq!(history.dividends[0].amount, 0.24);
        assert_eq!(history.splits.len(), 1);
        assert_eq!(history.splits[0].ratio, "4:1");
    }

    #[test]
    fn reports_api_errors() {
        let response: YahooResponse = serde_json::from_str(
            r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found"}}}"#,
        )
        .unwrap();

        let error = response_to_market_history(response).unwrap_err();

        assert_eq!(
            error,
            YahooParseError::Api {
                code: "Not Found".to_string(),
                description: "No data found".to_string(),
            }
        );
    }

    #[test]
    fn reports_missing_quotes() {
        let response: YahooResponse = serde_json::from_str(
            r#"
            {
              "chart": {
                "result": [{
                  "meta": {"symbol": "AAPL"},
                  "timestamp": [],
                  "indicators": {"quote": [], "adjclose": []},
                  "events": null
                }],
                "error": null
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            response_to_market_history(response).unwrap_err(),
            YahooParseError::MissingQuotes
        );
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente da Yahoo Finance"]
    async fn downloads_and_maps_real_data() {
        let data_inicio = Utc::now().date_naive() - chrono::Duration::days(14);
        let response = super::super::client::download_historico("AAPL", data_inicio)
            .await
            .expect("a Yahoo deve devolver uma resposta válida");

        let history = response_to_market_history(response)
            .expect("o DTO real da Yahoo deve ser convertido para o domínio");

        assert_eq!(history.ticker, "AAPL");
        assert!(!history.daily_quotes.is_empty());
        assert!(
            history
                .daily_quotes
                .iter()
                .any(|quote| quote.close.is_some())
        );
    }
}
