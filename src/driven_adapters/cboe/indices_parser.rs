use chrono::NaiveDate;
use std::error::Error;
use std::fmt;

use super::indices_client::{CboeIndexResponse, CboeIndexRowRaw};
use crate::hexagon::domain::index_history::{DailyIndexPrice, IndexHistory};

#[derive(Debug, PartialEq)]
pub enum CboeIndexParseError {
    InvalidDate(String),
    InvalidNumber { field: &'static str, value: String },
}

impl fmt::Display for CboeIndexParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate(value) => write!(formatter, "data CBOE inválida: {value}"),
            Self::InvalidNumber { field, value } => {
                write!(formatter, "valor CBOE inválido em {field}: {value}")
            }
        }
    }
}

impl Error for CboeIndexParseError {}

pub fn response_to_index_history(
    response: CboeIndexResponse,
) -> Result<IndexHistory, CboeIndexParseError> {
    let mut daily_prices = response
        .rows
        .into_iter()
        .map(row_to_daily_price)
        .collect::<Result<Vec<_>, _>>()?;
    daily_prices.sort_by_key(|price| price.date);

    Ok(IndexHistory {
        ticker: response.ticker,
        daily_prices,
    })
}

fn row_to_daily_price(row: CboeIndexRowRaw) -> Result<DailyIndexPrice, CboeIndexParseError> {
    Ok(DailyIndexPrice {
        date: NaiveDate::parse_from_str(&row.date, "%m/%d/%Y")
            .map_err(|_| CboeIndexParseError::InvalidDate(row.date))?,
        open: parse_optional_number("open", row.open)?,
        high: parse_optional_number("high", row.high)?,
        low: parse_optional_number("low", row.low)?,
        close: parse_number("close", row.close)?,
    })
}

fn parse_optional_number(
    field: &'static str,
    value: Option<String>,
) -> Result<Option<f64>, CboeIndexParseError> {
    value.map(|value| parse_number(field, value)).transpose()
}

fn parse_number(field: &'static str, value: String) -> Result<f64, CboeIndexParseError> {
    value
        .parse()
        .map_err(|_| CboeIndexParseError::InvalidNumber { field, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cboe_dto_to_domain() {
        let response = CboeIndexResponse {
            ticker: "VIX".to_string(),
            rows: vec![CboeIndexRowRaw {
                date: "01/02/1990".to_string(),
                open: Some("17.24".to_string()),
                high: Some("18.00".to_string()),
                low: Some("16.50".to_string()),
                close: "17.50".to_string(),
            }],
        };

        let history = response_to_index_history(response).unwrap();

        assert_eq!(history.ticker, "VIX");
        assert_eq!(history.daily_prices.len(), 1);
        assert_eq!(
            history.daily_prices[0].date,
            NaiveDate::from_ymd_opt(1990, 1, 2).unwrap()
        );
        assert_eq!(history.daily_prices[0].close, 17.50);
    }

    #[test]
    fn reports_invalid_numbers() {
        let response = CboeIndexResponse {
            ticker: "VIX".to_string(),
            rows: vec![CboeIndexRowRaw {
                date: "01/02/1990".to_string(),
                open: Some("invalid".to_string()),
                high: Some("18.00".to_string()),
                low: Some("16.50".to_string()),
                close: "17.50".to_string(),
            }],
        };

        assert_eq!(
            response_to_index_history(response).unwrap_err(),
            CboeIndexParseError::InvalidNumber {
                field: "open",
                value: "invalid".to_string(),
            }
        );
    }

    #[tokio::test]
    #[ignore = "teste de integração dependente da CBOE"]
    async fn downloads_and_maps_real_index() {
        let response = super::super::indices_client::download_indice("VIX")
            .await
            .expect("a CBOE deve devolver o CSV do VIX");

        let history = response_to_index_history(response)
            .expect("o CSV real da CBOE deve ser convertido para o domínio");

        assert_eq!(history.ticker, "VIX");
        assert!(!history.daily_prices.is_empty());
        assert!(
            history
                .daily_prices
                .windows(2)
                .all(|pair| pair[0].date <= pair[1].date)
        );
    }
}
