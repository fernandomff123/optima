//! Gamma-exposure use case and snapshot-source selection.

use async_trait::async_trait;
use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};

use crate::hexagon::{
    PortError, PortResult,
    domain::{
        gamma_exposure::{
            GammaExposureAnalysis, ModeledExpirationInput, SnapshotOrigin, calculate,
            modeled_profile,
        },
        interest_rates::BoundedCubicSpline,
        option_volatility::{calculate_forward, is_pm_settlement},
        options::Snapshot,
    },
    driven_ports::{
        for_consulting_trading_calendar::ForConsultingTradingCalendar,
        for_loading_option_chains::ForLoadingOptionChains,
        for_loading_yield_curves::ForLoadingYieldCurves,
        for_obtaining_option_chains::ForObtainingOptionChains,
        for_resolving_option_contract_specifications::ForResolvingOptionContractSpecifications,
    },
    driving_ports::for_viewing_gamma_exposure::{ForViewingGammaExposure, GammaExposureRequest},
};

use super::option_snapshot_enrichment::OptionSnapshotEnrichment;

pub struct GammaExposureApplication<
    Calendar,
    IntradayOptions,
    StoredOptions,
    Specifications,
    YieldCurves,
> {
    calendar: Calendar,
    intraday_options: IntradayOptions,
    stored_options: StoredOptions,
    intraday_enrichment: OptionSnapshotEnrichment<Specifications>,
    yield_curves: YieldCurves,
}

impl<Calendar, IntradayOptions, StoredOptions, Specifications, YieldCurves>
    GammaExposureApplication<Calendar, IntradayOptions, StoredOptions, Specifications, YieldCurves>
{
    pub fn new(
        calendar: Calendar,
        intraday_options: IntradayOptions,
        stored_options: StoredOptions,
        specifications: Specifications,
        yield_curves: YieldCurves,
    ) -> Self {
        Self {
            calendar,
            intraday_options,
            stored_options,
            intraday_enrichment: OptionSnapshotEnrichment::new(specifications),
            yield_curves,
        }
    }

    #[cfg(test)]
    pub(crate) fn contract_specifications(&self) -> &Specifications {
        self.intraday_enrichment.contract_specifications()
    }
}

#[async_trait]
impl<Calendar, IntradayOptions, StoredOptions, Specifications, YieldCurves> ForViewingGammaExposure
    for GammaExposureApplication<
        Calendar,
        IntradayOptions,
        StoredOptions,
        Specifications,
        YieldCurves,
    >
where
    Calendar: ForConsultingTradingCalendar,
    IntradayOptions: ForObtainingOptionChains,
    StoredOptions: ForLoadingOptionChains,
    Specifications: ForResolvingOptionContractSpecifications,
    YieldCurves: ForLoadingYieldCurves,
{
    async fn gamma_exposure(
        &self,
        request: GammaExposureRequest,
    ) -> PortResult<GammaExposureAnalysis> {
        validate_profile_request(request.range_percent, request.points)?;
        let ticker = normalized_ticker(&request.ticker)?;
        let (snapshot, origin, valuation_time) =
            if self.calendar.is_regular_session(request.valuation_time)? {
                let mut snapshot = self.intraday_options.obtain_option_chain(&ticker).await?;
                self.intraday_enrichment.enrich(&mut snapshot).await?;
                (snapshot, SnapshotOrigin::Intraday, request.valuation_time)
            } else {
                let stored = self
                    .stored_options
                    .load_option_chain(&ticker)
                    .await?
                    .ok_or_else(|| {
                        PortError::Unavailable(format!(
                            "end-of-day option snapshot for '{ticker}' is unavailable"
                        ))
                    })?;
                let valuation_time = self.calendar.session_close(stored.session_date)?;
                (stored.snapshot, SnapshotOrigin::EndOfDay, valuation_time)
            };
        let exposure = calculate(&snapshot, origin);
        if exposure.diagnostics.included_contracts == 0 {
            return Err(PortError::Unavailable(format!(
                "gamma exposure for '{ticker}' is unavailable because no contracts are eligible"
            )));
        }
        let expiration_inputs = self
            .modeled_expiration_inputs(&snapshot, valuation_time)
            .await?;
        let modeled_profile = modeled_profile(
            &snapshot,
            valuation_time,
            request.range_percent,
            request.points,
            &expiration_inputs,
        )
        .map_err(|message| PortError::Unavailable(message.into()))?;
        Ok(GammaExposureAnalysis {
            current_exposure: exposure,
            modeled_profile,
        })
    }
}

impl<Calendar, IntradayOptions, StoredOptions, Specifications, YieldCurves>
    GammaExposureApplication<Calendar, IntradayOptions, StoredOptions, Specifications, YieldCurves>
where
    Calendar: ForConsultingTradingCalendar,
    YieldCurves: ForLoadingYieldCurves,
{
    async fn modeled_expiration_inputs(
        &self,
        snapshot: &Snapshot,
        valuation_time: DateTime<Utc>,
    ) -> PortResult<BTreeMap<(String, NaiveDate), ModeledExpirationInput>> {
        let curve = self
            .yield_curves
            .load_yield_curve(valuation_time.date_naive())
            .await?
            .ok_or_else(|| PortError::Unavailable("yield curve is unavailable".into()))?;
        let spline = BoundedCubicSpline::from_treasury_curve(&curve)
            .map_err(|error| PortError::Unavailable(error.to_string()))?;
        let spot = snapshot
            .underlying_price
            .as_ref()
            .map(|price| price.value)
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| PortError::Unavailable("spot is unavailable".into()))?;
        let mut inputs = BTreeMap::new();
        for chain in &snapshot.chains {
            let mut expirations = chain
                .contratos
                .iter()
                .map(|contract| contract.expiration)
                .collect::<Vec<_>>();
            expirations.sort_unstable();
            expirations.dedup();
            for expiration in expirations {
                let expiration_time = if is_pm_settlement(&chain.root) {
                    self.calendar.session_close(expiration)
                } else {
                    self.calendar.session_open(expiration)
                };
                let Ok(expiration_time) = expiration_time else {
                    continue;
                };
                let minutes = expiration_time
                    .signed_duration_since(valuation_time)
                    .num_minutes();
                let time_to_expiration = minutes as f64 / 525_600.0;
                if time_to_expiration <= 0.0 {
                    continue;
                }
                let rate_days = expiration.signed_duration_since(curve.date).num_days() as f64;
                let Ok(interest_rate) = spline.continuously_compounded_rate(rate_days) else {
                    continue;
                };
                let Ok(forward) = calculate_forward(
                    &chain.contratos,
                    expiration,
                    interest_rate,
                    time_to_expiration,
                ) else {
                    continue;
                };
                let dividend_yield = interest_rate - (forward / spot).ln() / time_to_expiration;
                if !dividend_yield.is_finite() {
                    continue;
                }
                inputs.insert(
                    (chain.root.clone(), expiration),
                    ModeledExpirationInput {
                        time_to_expiration,
                        interest_rate,
                        dividend_yield,
                    },
                );
            }
        }
        Ok(inputs)
    }
}

fn validate_profile_request(range_percent: f64, points: usize) -> PortResult<()> {
    if !range_percent.is_finite() || !(5.0..=50.0).contains(&range_percent) {
        return Err(PortError::InvalidRequest(
            "range_percent must be between 5 and 50".into(),
        ));
    }
    if !(21..=201).contains(&points) || points.is_multiple_of(2) {
        return Err(PortError::InvalidRequest(
            "points must be an odd number between 21 and 201".into(),
        ));
    }
    Ok(())
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
