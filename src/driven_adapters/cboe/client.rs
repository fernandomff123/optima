//! HTTP client and wire DTOs for Cboe delayed option quotes.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CboeResponse {
    pub timestamp: String,
    pub data: CboeData,
}

#[derive(Debug, Deserialize)]
pub struct CboeData {
    pub options: Vec<CboeOptionRowRaw>,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub last_trade_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CboeOptionRowRaw {
    pub option: String,
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
    pub open_interest: Option<f64>,
    pub delta: f64,
    pub gamma: Option<f64>,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
    pub theo: f64,
    #[serde(default)]
    pub iv: f64,
}

const CBOE_URL: &str = "https://cdn.cboe.com/api/global/delayed_quotes/options";

pub async fn download_snapshot(
    ticker: &str,
) -> Result<CboeResponse, Box<dyn std::error::Error + Send + Sync>> {
    let symbol = cboe_symbol(ticker);
    let url = format!("{CBOE_URL}/{symbol}.json");
    let cliente = reqwest::Client::new();

    let response: CboeResponse = cliente
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept-Encoding", "identity")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response)
}

fn cboe_symbol(ticker: &str) -> &str {
    if ticker.trim().eq_ignore_ascii_case("SPX") {
        "_SPX"
    } else {
        ticker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_spx_to_the_cboe_api_symbol() {
        assert_eq!(cboe_symbol("SPX"), "_SPX");
        assert_eq!(cboe_symbol(" spx "), "_SPX");
        assert_eq!(cboe_symbol("AAPL"), "AAPL");
    }
}
