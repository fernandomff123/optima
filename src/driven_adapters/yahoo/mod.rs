//! Driven adapter for the external historical-market-data actor.

mod client;
pub mod live;
mod parser;

use async_trait::async_trait;
use chrono::NaiveDate;
use std::time::Duration;

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        live_price::LivePrice,
        market_history::MarketHistory,
        tracked_ticker::{ResolvedUnderlying, UnderlyingMetadata},
    },
    driven_ports::{
        for_obtaining_live_prices::ForObtainingLivePrices,
        for_obtaining_market_history::ForObtainingMarketHistory,
        for_resolving_underlying_symbols::{
            ForResolvingUnderlyingSymbols, UnderlyingResolutionError,
        },
        for_streaming_live_prices::ForStreamingLivePrices,
    },
};

/// Obtains historical prices and corporate actions from Yahoo Finance.
///
/// The provider name belongs to this adapter. The port implemented here is
/// deliberately provider-neutral and describes only what the application needs.
#[derive(Debug, Default, Clone, Copy)]
pub struct YahooMarketHistoryAdapter;

/// Obtains a current price from Yahoo Finance.
#[derive(Debug, Default, Clone, Copy)]
pub struct YahooLivePricesAdapter;

#[derive(Debug, Clone)]
pub struct YahooUnderlyingResolverAdapter {
    client: reqwest::Client,
    chart_url: String,
}

impl Default for YahooUnderlyingResolverAdapter {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
            chart_url: "https://query1.finance.yahoo.com/v8/finance/chart".to_string(),
        }
    }
}

#[async_trait]
impl ForResolvingUnderlyingSymbols for YahooUnderlyingResolverAdapter {
    async fn resolve_underlying(
        &self,
        ticker: &str,
    ) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
        let ticker = client::normalize_ticker(ticker).map_err(|error| {
            UnderlyingResolutionError::InvalidProviderResponse(error.to_string())
        })?;
        let provider_symbol = chart_provider_symbol(&ticker);
        let url = format!("{}/{provider_symbol}", self.chart_url.trim_end_matches('/'));
        let response = self
            .client
            .get(url)
            .timeout(Duration::from_secs(30))
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
            .query(&[("range", "1d"), ("interval", "1d")])
            .send()
            .await
            .map_err(|error| temporary_transport_error(error.to_string()))?;
        let status = response.status();
        classify_resolution_status(status, &ticker)?;
        let body = response
            .bytes()
            .await
            .map_err(|error| temporary_transport_error(error.to_string()))?;
        let response = parse_resolution_response(&body)?;
        map_resolved_underlying(&ticker, &provider_symbol, response)
    }
}

fn map_resolved_underlying(
    ticker: &str,
    provider_symbol: &str,
    response: client::YahooResponse,
) -> Result<ResolvedUnderlying, UnderlyingResolutionError> {
    if let Some(error) = response.chart.error {
        if error.code.eq_ignore_ascii_case("not found") {
            return Err(UnderlyingResolutionError::NotFound(format!(
                "underlying {ticker} was not found: {}",
                error.description
            )));
        }
        return Err(UnderlyingResolutionError::InvalidProviderResponse(format!(
            "Yahoo API error {}: {}",
            error.code, error.description
        )));
    }
    let result = response
        .chart
        .result
        .and_then(|mut results| results.drain(..).next())
        .ok_or_else(|| {
            UnderlyingResolutionError::InvalidProviderResponse(
                "Yahoo response did not contain a chart result".to_string(),
            )
        })?;
    if !result
        .meta
        .symbol
        .trim()
        .eq_ignore_ascii_case(provider_symbol)
    {
        return Err(UnderlyingResolutionError::InvalidProviderResponse(
            "Yahoo response symbol did not match the requested ticker".to_string(),
        ));
    }
    Ok(ResolvedUnderlying {
        ticker: ticker.to_string(),
        metadata: UnderlyingMetadata {
            currency: result.meta.currency,
            exchange: result.meta.exchange_name,
            timezone: result.meta.exchange_timezone_name,
            instrument_type: result.meta.instrument_type,
        },
    })
}

fn chart_provider_symbol(ticker: &str) -> String {
    match ticker {
        "BRK.B" => "BRK-B".to_string(),
        "SPX" => "^GSPC".to_string(),
        _ => ticker.to_string(),
    }
}

fn temporary_transport_error(message: String) -> UnderlyingResolutionError {
    UnderlyingResolutionError::TemporarilyUnavailable(message)
}

fn classify_resolution_status(
    status: reqwest::StatusCode,
    ticker: &str,
) -> Result<(), UnderlyingResolutionError> {
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(UnderlyingResolutionError::NotFound(format!(
            "underlying {ticker} was not found"
        )));
    }
    if matches!(
        status,
        reqwest::StatusCode::REQUEST_TIMEOUT | reqwest::StatusCode::TOO_MANY_REQUESTS
    ) || status.is_server_error()
    {
        return Err(UnderlyingResolutionError::TemporarilyUnavailable(format!(
            "Yahoo returned HTTP {status}"
        )));
    }
    if !status.is_success() {
        return Err(UnderlyingResolutionError::InvalidProviderResponse(format!(
            "Yahoo returned unexpected HTTP {status}"
        )));
    }
    Ok(())
}

fn parse_resolution_response(
    body: &[u8],
) -> Result<client::YahooResponse, UnderlyingResolutionError> {
    serde_json::from_slice(body)
        .map_err(|error| UnderlyingResolutionError::InvalidProviderResponse(error.to_string()))
}

#[async_trait]
impl ForObtainingMarketHistory for YahooMarketHistoryAdapter {
    async fn obtain_market_history(
        &self,
        ticker: &str,
        since: NaiveDate,
    ) -> PortResult<MarketHistory> {
        let ticker = ticker.trim().to_ascii_uppercase();
        let provider_symbol = chart_provider_symbol(&ticker);

        let response = client::download_historico(&provider_symbol, since)
            .await
            .map_err(unavailable)?;
        let mut history = parser::response_to_market_history(response).map_err(unavailable)?;

        // Keep provider-specific aliases from leaking into the domain model.
        history.ticker = ticker;
        Ok(history)
    }
}

#[async_trait]
impl ForObtainingLivePrices for YahooLivePricesAdapter {
    async fn obtain_live_price(&self, ticker: &str) -> PortResult<LivePrice> {
        let price = live::fetch_price(ticker).await.map_err(unavailable)?;
        Ok(LivePrice {
            ticker: price.ticker,
            price: price.price,
            market_time: price.time,
            currency: price.currency,
            exchange: price.exchange,
            regular_session: price.market_hours == 1,
            change: price.change,
            change_percent: price.change_percent,
            day_volume: price.day_volume,
        })
    }
}

#[async_trait]
impl ForStreamingLivePrices for YahooLivePricesAdapter {
    async fn stream_live_prices(
        &self,
        subscriptions: tokio::sync::watch::Receiver<String>,
        prices: tokio::sync::mpsc::Sender<LivePrice>,
    ) -> PortResult<()> {
        let (provider_prices, mut received_provider_prices) = tokio::sync::mpsc::channel(32);
        let stream = live::stream_prices(subscriptions, provider_prices);
        let forward = async move {
            while let Some(price) = received_provider_prices.recv().await {
                if prices
                    .send(LivePrice {
                        ticker: price.ticker,
                        price: price.price,
                        market_time: price.time,
                        currency: price.currency,
                        exchange: price.exchange,
                        regular_session: price.market_hours == 1,
                        change: price.change,
                        change_percent: price.change_percent,
                        day_volume: price.day_volume,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };
        tokio::try_join!(async { stream.await.map_err(unavailable) }, async {
            forward.await;
            Ok::<(), PortError>(())
        })?;
        Ok(())
    }
}

fn unavailable(error: impl std::fmt::Display) -> PortError {
    PortError::Unavailable(error.to_string())
}

pub(crate) use client::{download_atualizacao, normalize_ticker};

#[cfg(test)]
mod resolver_tests {
    use super::*;

    fn response_with_symbol(symbol: &str) -> client::YahooResponse {
        serde_json::from_value(serde_json::json!({
            "chart": {
                "result": [{
                    "meta": {
                        "symbol": symbol,
                        "currency": "USD",
                        "exchangeName": "NMS",
                        "exchangeTimezoneName": "America/New_York",
                        "instrumentType": "EQUITY"
                    },
                    "timestamp": [],
                    "indicators": {"quote": [], "adjclose": []},
                    "events": null
                }],
                "error": null
            }
        }))
        .unwrap()
    }

    fn success_response() -> &'static [u8] {
        br#"{"chart":{"result":[{"meta":{"symbol":"MSFT","currency":"USD","exchangeName":"NMS","exchangeTimezoneName":"America/New_York","instrumentType":"EQUITY"},"timestamp":[],"indicators":{"quote":[],"adjclose":[]},"events":null}],"error":null}}"#
    }

    #[test]
    fn deserializes_confirmed_symbol_and_available_chart_metadata() {
        let response = parse_resolution_response(success_response()).unwrap();
        let meta = &response.chart.result.unwrap()[0].meta;
        assert_eq!(meta.symbol, "MSFT");
        assert_eq!(meta.currency.as_deref(), Some("USD"));
        assert_eq!(meta.exchange_name.as_deref(), Some("NMS"));
        assert_eq!(meta.instrument_type.as_deref(), Some("EQUITY"));
    }

    #[test]
    fn keeps_public_identity_while_matching_explicit_provider_symbols() {
        for (input, public_ticker, provider_symbol) in [
            (" msft ", "MSFT", "MSFT"),
            (" brk.b ", "BRK.B", "BRK-B"),
            (" spx ", "SPX", "^GSPC"),
        ] {
            let normalized = client::normalize_ticker(input).unwrap();
            assert_eq!(normalized, public_ticker);
            assert_eq!(chart_provider_symbol(&normalized), provider_symbol);
            let resolved = map_resolved_underlying(
                &normalized,
                provider_symbol,
                response_with_symbol(provider_symbol),
            )
            .unwrap();
            assert_eq!(resolved.ticker, public_ticker);
        }
    }

    #[test]
    fn rejects_a_genuinely_different_provider_symbol() {
        assert!(matches!(
            map_resolved_underlying("MSFT", "MSFT", response_with_symbol("AAPL")),
            Err(UnderlyingResolutionError::InvalidProviderResponse(_))
        ));
    }

    #[test]
    fn classifies_not_found_rate_limit_and_server_errors() {
        for (status, expected) in [
            (reqwest::StatusCode::NOT_FOUND, "not_found"),
            (reqwest::StatusCode::TOO_MANY_REQUESTS, "temporary"),
            (reqwest::StatusCode::REQUEST_TIMEOUT, "temporary"),
            (reqwest::StatusCode::INTERNAL_SERVER_ERROR, "temporary"),
            (reqwest::StatusCode::SERVICE_UNAVAILABLE, "temporary"),
        ] {
            let error = classify_resolution_status(status, "MSFT").unwrap_err();
            assert!(matches!(
                (expected, error),
                ("not_found", UnderlyingResolutionError::NotFound(_))
                    | (
                        "temporary",
                        UnderlyingResolutionError::TemporarilyUnavailable(_)
                    )
            ));
        }
    }

    #[test]
    fn classifies_transport_errors_as_temporary_and_invalid_json_as_incompatible() {
        let error = temporary_transport_error("request timed out".into());
        assert!(matches!(
            error,
            UnderlyingResolutionError::TemporarilyUnavailable(_)
        ));
        let error = temporary_transport_error("connection reset".into());
        assert!(matches!(
            error,
            UnderlyingResolutionError::TemporarilyUnavailable(_)
        ));
        assert!(matches!(
            parse_resolution_response(b"not-json"),
            Err(UnderlyingResolutionError::InvalidProviderResponse(_))
        ));
    }
}
