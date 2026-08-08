//! Option-analysis use cases.

use async_trait::async_trait;
use chrono::NaiveDate;

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        option_volatility,
        options::Snapshot,
        simulation::Greeks,
        volatility::TermStructure,
        volatility_surface::{VolatilitySkew, VolatilitySurface},
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_reference_prices::ForLoadingReferencePrices,
        for_loading_volatility_term_structures::ForLoadingVolatilityTermStructures,
        for_loading_yield_curves::ForLoadingYieldCurves,
    },
    driving_ports::for_analyzing_options::{ForAnalyzingOptions, GreeksRequest},
};

pub struct OptionsApplication<OptionChains, OptionDataStore, YieldCurves, TradingCalendar> {
    option_chains: OptionChains,
    option_data_store: OptionDataStore,
    yield_curves: YieldCurves,
    trading_calendar: TradingCalendar,
}

impl<OptionChains, OptionDataStore, YieldCurves, TradingCalendar>
    OptionsApplication<OptionChains, OptionDataStore, YieldCurves, TradingCalendar>
{
    pub fn new(
        option_chains: OptionChains,
        option_data_store: OptionDataStore,
        yield_curves: YieldCurves,
        trading_calendar: TradingCalendar,
    ) -> Self {
        Self {
            option_chains,
            option_data_store,
            yield_curves,
            trading_calendar,
        }
    }
}

impl<OptionChains, OptionDataStore, YieldCurves, TradingCalendar>
    OptionsApplication<OptionChains, OptionDataStore, YieldCurves, TradingCalendar>
where
    OptionChains: ForLoadingOptionChains,
{
    async fn snapshot(&self, ticker: &str) -> PortResult<Snapshot> {
        let ticker = normalized_ticker(ticker)?;
        self.option_chains
            .load_option_chain(&ticker)
            .await?
            .ok_or_else(|| {
                PortError::NotFound(format!("option chain for '{ticker}' was not found"))
            })
    }
}

#[async_trait]
impl<OptionChains, OptionDataStore, YieldCurves, TradingCalendar> ForAnalyzingOptions
    for OptionsApplication<OptionChains, OptionDataStore, YieldCurves, TradingCalendar>
where
    OptionChains: ForLoadingOptionChains,
    OptionDataStore: ForLoadingVolatilityTermStructures + ForLoadingReferencePrices,
    YieldCurves: ForLoadingYieldCurves,
    TradingCalendar: ForConsultingTradingCalendar,
{
    async fn option_chain(&self, ticker: &str) -> PortResult<Snapshot> {
        self.snapshot(ticker).await
    }

    async fn term_structure(&self, ticker: &str) -> PortResult<TermStructure> {
        let ticker = normalized_ticker(ticker)?;
        if let Some(stored) = self.option_data_store.load_term_structure(&ticker).await? {
            return Ok(stored);
        }
        build_term_structure(
            &self.option_chains,
            &self.yield_curves,
            &self.trading_calendar,
            &ticker,
        )
        .await
    }

    async fn volatility_surface(&self, ticker: &str) -> PortResult<VolatilitySurface> {
        let snapshot = self.snapshot(ticker).await?;
        let reference_price = self
            .option_data_store
            .load_reference_price(&snapshot.ticker)
            .await?
            .ok_or_else(|| {
                PortError::NotFound(format!(
                    "reference price for '{}' was not found",
                    snapshot.ticker
                ))
            })?;
        VolatilitySurface::from_snapshot(&snapshot, reference_price).ok_or_else(|| {
            PortError::Unavailable(format!(
                "a volatility surface could not be built for '{}'",
                snapshot.ticker
            ))
        })
    }

    async fn volatility_skew(
        &self,
        ticker: &str,
        expiration: NaiveDate,
    ) -> PortResult<VolatilitySkew> {
        let surface = self.volatility_surface(ticker).await?;
        let points = surface
            .points
            .into_iter()
            .filter(|point| point.expiration == expiration)
            .collect::<Vec<_>>();
        if points.is_empty() {
            return Err(PortError::NotFound(format!(
                "volatility skew for '{}' at {expiration} was not found",
                surface.ticker
            )));
        }
        Ok(VolatilitySkew {
            ticker: surface.ticker,
            expiration,
            points,
        })
    }

    async fn greeks(&self, request: GreeksRequest) -> PortResult<Greeks> {
        let snapshot = self.snapshot(&request.ticker).await?;
        let contract = snapshot
            .contratos
            .iter()
            .find(|contract| contract.occ_symbol == request.occ_symbol)
            .ok_or_else(|| {
                PortError::NotFound(format!(
                    "option contract '{}' was not found",
                    request.occ_symbol
                ))
            })?;
        Ok(Greeks {
            delta: contract.delta,
            gamma: contract.gamma,
            vega: contract.vega,
            theta: contract.theta,
            rho: contract.rho,
        })
    }
}

/// Builds option analytics from required ports without knowing their adapters.
pub(crate) async fn build_term_structure<OptionChains, YieldCurves, TradingCalendar>(
    option_chains: &OptionChains,
    yield_curves: &YieldCurves,
    trading_calendar: &TradingCalendar,
    ticker: &str,
) -> PortResult<TermStructure>
where
    OptionChains: ForLoadingOptionChains,
    YieldCurves: ForLoadingYieldCurves,
    TradingCalendar: ForConsultingTradingCalendar,
{
    let ticker = normalized_ticker(ticker)?;
    let snapshot = option_chains
        .load_option_chain(&ticker)
        .await?
        .ok_or_else(|| PortError::NotFound(format!("option chain for '{ticker}' was not found")))?;
    let curve = yield_curves
        .load_yield_curve(snapshot.timestamp_utc.date_naive())
        .await?
        .ok_or_else(|| PortError::NotFound(format!("yield curve for '{ticker}' was not found")))?;
    let base_time = if trading_calendar.is_regular_session(snapshot.timestamp_utc)? {
        snapshot.timestamp_utc
    } else {
        trading_calendar.latest_session_close_before(snapshot.timestamp_utc)?
    };

    option_volatility::calculate_term_structure(&snapshot, &curve, |expiration, is_pm| {
        let expiration_time = if is_pm {
            trading_calendar.session_close(expiration)
        } else {
            trading_calendar.session_open(expiration)
        }
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
        let minutes = expiration_time
            .signed_duration_since(base_time)
            .num_minutes();
        Ok(minutes as f64 / 525_600.0)
    })
    .map_err(|error| PortError::Unavailable(error.to_string()))
}

fn normalized_ticker(ticker: &str) -> PortResult<String> {
    let ticker = ticker.trim().to_ascii_uppercase();
    if ticker.is_empty()
        || !ticker
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '^')
    {
        return Err(PortError::InvalidRequest(
            "ticker must contain only ASCII letters, digits, or '^'".into(),
        ));
    }
    Ok(ticker)
}
