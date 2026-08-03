//! HTTP client and XML wire DTOs for U.S. Treasury yield curves.

use serde::Deserialize;
use std::error::Error;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct TreasuryFeed {
    #[serde(rename = "entry", default)]
    pub entries: Vec<TreasuryEntry>,
}

#[derive(Debug, Deserialize)]
pub struct TreasuryEntry {
    #[serde(rename = "content")]
    pub content: TreasuryContent,
}

#[derive(Debug, Deserialize)]
pub struct TreasuryContent {
    #[serde(rename = "properties")]
    pub properties: TreasuryProperties,
}

#[derive(Debug, Deserialize)]
pub struct TreasuryProperties {
    #[serde(rename = "NEW_DATE")]
    pub date: String,
    #[serde(rename = "BC_1MONTH")]
    pub m1: Option<f64>,
    #[serde(rename = "BC_2MONTH")]
    pub m2: Option<f64>,
    #[serde(rename = "BC_3MONTH")]
    pub m3: Option<f64>,
    #[serde(rename = "BC_6MONTH")]
    pub m6: Option<f64>,
    #[serde(rename = "BC_1YEAR")]
    pub y1: Option<f64>,
    #[serde(rename = "BC_5YEAR")]
    pub y5: Option<f64>,
    #[serde(rename = "BC_2YEAR")]
    pub y2: Option<f64>,
    #[serde(rename = "BC_3YEAR")]
    pub y3: Option<f64>,
    #[serde(rename = "BC_10YEAR")]
    pub y10: Option<f64>,
    #[serde(rename = "BC_7YEAR")]
    pub y7: Option<f64>,
    #[serde(rename = "BC_20YEAR")]
    pub y20: Option<f64>,
    #[serde(rename = "BC_30YEAR")]
    pub y30: Option<f64>,
}

const TREASURY_YEAR_URL: &str = "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xmlview?data=daily_treasury_yield_curve&field_tdr_date_value";

pub async fn download_ano(ano: &str) -> Result<TreasuryFeed, Box<dyn Error>> {
    let ano = normalize_ano(ano)?;
    download_feed(TREASURY_YEAR_URL, &ano).await
}

async fn download_feed(base_url: &str, value: &str) -> Result<TreasuryFeed, Box<dyn Error>> {
    let url = format!("{base_url}={value}");
    let cliente = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let resposta = cliente.get(&url).send().await?;
    if !resposta.status().is_success() {
        return Err(format!("Erro HTTP do Tesouro US em {url}: {}", resposta.status()).into());
    }

    let xml_content = resposta.text().await?;
    let feed: TreasuryFeed = quick_xml::de::from_str(&xml_content)?;

    Ok(feed)
}

fn normalize_ano(value: &str) -> Result<String, Box<dyn Error>> {
    if !value.chars().all(|c| c.is_ascii_digit()) {
        return Err("ano deve conter apenas dígitos".into());
    }

    match value.len() {
        2 => Ok(format!("20{value}")),
        4 => Ok(value.to_string()),
        _ => Err("ano deve estar no formato YY ou YYYY".into()),
    }
}
