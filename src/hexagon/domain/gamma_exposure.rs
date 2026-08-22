//! Pure gamma-exposure calculation over a factual option-chain snapshot.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, Utc};

use super::options::{OptionType, ProviderTimestampTimezone, Snapshot};
use super::simulation::black_scholes_gamma;

pub const EXCLUSION_SAMPLE_LIMIT: usize = 20;
pub const METHODOLOGY: &str =
    "gamma × open_interest × contract_multiplier × spot² × 0.01 (per 1% spot move)";
pub const SIGN_CONVENTION: &str = "Analytical assumption: calls are positive and puts are negative; this is not an observed market fact.";
pub const PROFILE_METHODOLOGY: &str = "Black-Scholes gamma at each hypothetical spot, then gamma × open_interest × contract_multiplier × spot² × 0.01";
pub const STICKY_STRIKE_ASSUMPTION: &str = "Model assumption: each contract's snapshot implied volatility remains constant by strike (sticky-strike).";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotOrigin {
    Intraday,
    EndOfDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExclusionReason {
    MissingSpot,
    InvalidSpot,
    MissingGamma,
    InvalidGamma,
    MissingOpenInterest,
    InvalidOpenInterest,
    MissingMultiplier,
    InvalidMultiplier,
    InvalidStrike,
    ExpiredContract,
    MissingImpliedVolatility,
    InvalidImpliedVolatility,
    MissingForwardCarry,
    NumericOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExclusionSample {
    pub occ_symbol: String,
    pub reasons: Vec<ExclusionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GammaExposureDiagnostics {
    pub total_contracts: u64,
    pub included_contracts: u64,
    pub excluded_contracts: u64,
    pub excluded_by_reason: BTreeMap<ExclusionReason, u64>,
    pub samples: Vec<ExclusionSample>,
    pub sample_limit: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GammaExposureBucket<K> {
    pub key: K,
    pub calls_gex: f64,
    pub puts_gex: f64,
    pub net_gex: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GammaExposure {
    pub ticker: String,
    pub spot: Option<f64>,
    pub currency: Option<String>,
    pub as_of: Option<DateTime<Utc>>,
    pub snapshot_origin: SnapshotOrigin,
    pub calls_gex: f64,
    pub puts_gex: f64,
    pub net_gex: f64,
    pub by_strike: Vec<GammaExposureBucket<f64>>,
    pub by_expiration: Vec<GammaExposureBucket<NaiveDate>>,
    pub methodology: &'static str,
    pub sign_convention: &'static str,
    pub diagnostics: GammaExposureDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModeledExpirationInput {
    pub time_to_expiration: f64,
    pub interest_rate: f64,
    pub dividend_yield: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModeledGammaExposurePoint {
    pub spot: f64,
    pub call_gex: f64,
    pub put_gex: f64,
    pub net_gex: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModeledGammaExposureProfile {
    pub valuation_time: DateTime<Utc>,
    pub range_percent: f64,
    pub points: usize,
    pub methodology: &'static str,
    pub sticky_strike_assumption: &'static str,
    pub included_contracts: u64,
    pub excluded_contracts: u64,
    pub excluded_by_reason: BTreeMap<ExclusionReason, u64>,
    pub samples: Vec<ExclusionSample>,
    pub sample_limit: usize,
    pub profile: Vec<ModeledGammaExposurePoint>,
    pub zero_crossings: Vec<f64>,
    pub nearest_zero_crossing: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GammaExposureAnalysis {
    pub current_exposure: GammaExposure,
    pub modeled_profile: Option<ModeledGammaExposureProfile>,
}

#[derive(Debug)]
struct ModeledContract {
    option_type: OptionType,
    strike: f64,
    volatility: f64,
    open_interest: f64,
    multiplier: f64,
    model: ModeledExpirationInput,
}

pub fn modeled_profile(
    snapshot: &Snapshot,
    valuation_time: DateTime<Utc>,
    range_percent: f64,
    points: usize,
    expiration_inputs: &BTreeMap<(String, NaiveDate), ModeledExpirationInput>,
    expired_expirations: &BTreeSet<(String, NaiveDate)>,
) -> Result<Option<ModeledGammaExposureProfile>, &'static str> {
    if !range_percent.is_finite() || !(5.0..=50.0).contains(&range_percent) {
        return Err("range_percent must be between 5 and 50");
    }
    if !(21..=201).contains(&points) || points.is_multiple_of(2) {
        return Err("points must be an odd number between 21 and 201");
    }
    let spot = snapshot
        .underlying_price
        .as_ref()
        .map(|price| price.value)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or("spot is unavailable")?;
    let fraction = range_percent / 100.0;
    let start = spot * (1.0 - fraction);
    let end = spot * (1.0 + fraction);
    if !start.is_finite() || !end.is_finite() || start <= 0.0 {
        return Err("spot grid is invalid");
    }
    let step = (end - start) / (points - 1) as f64;
    let spots = (0..points)
        .map(|index| {
            if index == points / 2 {
                spot
            } else {
                start + step * index as f64
            }
        })
        .collect::<Vec<_>>();
    let mut excluded_by_reason = BTreeMap::new();
    let mut samples = Vec::new();
    let mut eligible = Vec::new();
    for chain in &snapshot.chains {
        for contract in &chain.contratos {
            let expiration_identity = (chain.root.clone(), contract.expiration);
            let model = expiration_inputs.get(&expiration_identity);
            let reason = if expired_expirations.contains(&expiration_identity) {
                Some(ExclusionReason::ExpiredContract)
            } else if contract.implied_volatility.is_none() {
                Some(ExclusionReason::MissingImpliedVolatility)
            } else if contract
                .implied_volatility
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                Some(ExclusionReason::InvalidImpliedVolatility)
            } else if contract.open_interest.is_none() {
                Some(ExclusionReason::MissingOpenInterest)
            } else if contract
                .open_interest
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            {
                Some(ExclusionReason::InvalidOpenInterest)
            } else if contract.contract_specification.is_none() {
                Some(ExclusionReason::MissingMultiplier)
            } else if contract
                .contract_specification
                .as_ref()
                .is_some_and(|value| {
                    !value.contract_multiplier.is_finite() || value.contract_multiplier <= 0.0
                })
            {
                Some(ExclusionReason::InvalidMultiplier)
            } else if !contract.strike.is_finite() || contract.strike <= 0.0 {
                Some(ExclusionReason::InvalidStrike)
            } else if model.is_none() {
                Some(ExclusionReason::MissingForwardCarry)
            } else {
                None
            };
            if let Some(reason) = reason {
                record_exclusion(
                    contract.occ_symbol.clone(),
                    vec![reason],
                    &mut excluded_by_reason,
                    &mut samples,
                );
                continue;
            }
            let Some(model) = model.copied() else {
                continue;
            };
            let candidate = ModeledContract {
                option_type: contract.option_type,
                strike: contract.strike,
                volatility: contract.implied_volatility.unwrap_or_default(),
                open_interest: contract.open_interest.unwrap_or_default(),
                multiplier: contract
                    .contract_specification
                    .as_ref()
                    .map_or(0.0, |value| value.contract_multiplier),
                model,
            };
            if spots
                .iter()
                .any(|modeled_spot| modeled_signed_gex(&candidate, *modeled_spot).is_none())
            {
                record_exclusion(
                    contract.occ_symbol.clone(),
                    vec![ExclusionReason::NumericOverflow],
                    &mut excluded_by_reason,
                    &mut samples,
                );
            } else {
                eligible.push(candidate);
            }
        }
    }
    if eligible.is_empty() {
        return Ok(None);
    }
    let mut profile = Vec::with_capacity(points);
    for modeled_spot in spots {
        let mut calls = 0.0;
        let mut puts = 0.0;
        for contract in &eligible {
            let value = modeled_signed_gex(contract, modeled_spot)
                .ok_or("modeled gamma exposure overflow")?;
            match contract.option_type {
                OptionType::Call => calls += value,
                OptionType::Put => puts -= value,
            }
        }
        let net = calls + puts;
        if !calls.is_finite() || !puts.is_finite() || !net.is_finite() {
            return Err("modeled gamma exposure aggregate overflow");
        }
        profile.push(ModeledGammaExposurePoint {
            spot: modeled_spot,
            call_gex: calls,
            put_gex: puts,
            net_gex: net,
        });
    }
    let zero_crossings = zero_crossings(&profile);
    let nearest_zero_crossing = zero_crossings
        .iter()
        .copied()
        .min_by(|left, right| (left - spot).abs().total_cmp(&(right - spot).abs()));
    let included_contracts = eligible.len() as u64;
    Ok(Some(ModeledGammaExposureProfile {
        valuation_time,
        range_percent,
        points,
        methodology: PROFILE_METHODOLOGY,
        sticky_strike_assumption: STICKY_STRIKE_ASSUMPTION,
        included_contracts,
        excluded_contracts: snapshot.contratos.len() as u64 - included_contracts,
        excluded_by_reason,
        samples,
        sample_limit: EXCLUSION_SAMPLE_LIMIT,
        profile,
        zero_crossings,
        nearest_zero_crossing,
    }))
}

fn modeled_signed_gex(contract: &ModeledContract, spot: f64) -> Option<f64> {
    let gamma = black_scholes_gamma(
        spot,
        contract.strike,
        contract.model.time_to_expiration,
        contract.model.interest_rate,
        contract.model.dividend_yield,
        contract.volatility,
    )
    .ok()?;
    checked_gex(gamma, contract.open_interest, contract.multiplier, spot)
}

pub fn zero_crossings(points: &[ModeledGammaExposurePoint]) -> Vec<f64> {
    let mut crossings = Vec::new();
    for pair in points.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.net_gex == 0.0 {
            crossings.push(left.spot);
        } else if left.net_gex.signum() != right.net_gex.signum() {
            let crossing = left.spot
                - left.net_gex * (right.spot - left.spot) / (right.net_gex - left.net_gex);
            if crossing.is_finite() {
                crossings.push(crossing);
            }
        }
    }
    if let Some(last) = points.last().filter(|point| point.net_gex == 0.0) {
        crossings.push(last.spot);
    }
    crossings.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
    crossings
}

#[derive(Default, Clone, Copy)]
struct Totals {
    calls: f64,
    puts: f64,
}

impl Totals {
    fn added(self, option_type: OptionType, unsigned_gex: f64) -> Option<Self> {
        let mut candidate = self;
        match option_type {
            OptionType::Call => candidate.calls += unsigned_gex,
            OptionType::Put => candidate.puts -= unsigned_gex,
        }
        (candidate.calls.is_finite() && candidate.puts.is_finite() && candidate.net().is_finite())
            .then_some(candidate)
    }

    fn net(&self) -> f64 {
        self.calls + self.puts
    }
}

/// Calculates signed GEX without acquiring or persisting data.
pub fn calculate(snapshot: &Snapshot, origin: SnapshotOrigin) -> GammaExposure {
    let observed_spot = snapshot.underlying_price.as_ref().map(|price| price.value);
    let spot_reason = match observed_spot {
        None => Some(ExclusionReason::MissingSpot),
        Some(value) if !value.is_finite() || value <= 0.0 => Some(ExclusionReason::InvalidSpot),
        Some(_) => None,
    };
    let spot = observed_spot.filter(|value| value.is_finite() && *value > 0.0);
    let mut totals = Totals::default();
    let mut strikes: BTreeMap<u64, (f64, Totals)> = BTreeMap::new();
    let mut expirations: BTreeMap<NaiveDate, Totals> = BTreeMap::new();
    let mut excluded_by_reason = BTreeMap::new();
    let mut samples = Vec::new();
    let mut included = 0_u64;

    for contract in &snapshot.contratos {
        let mut reasons = Vec::new();
        if let Some(reason) = spot_reason {
            reasons.push(reason);
        }
        match contract.gamma {
            None => reasons.push(ExclusionReason::MissingGamma),
            Some(value) if !value.is_finite() || value < 0.0 => {
                reasons.push(ExclusionReason::InvalidGamma)
            }
            Some(_) => {}
        }
        match contract.open_interest {
            None => reasons.push(ExclusionReason::MissingOpenInterest),
            Some(value) if !value.is_finite() || value < 0.0 => {
                reasons.push(ExclusionReason::InvalidOpenInterest)
            }
            Some(_) => {}
        }
        match contract.contract_specification.as_ref() {
            None => reasons.push(ExclusionReason::MissingMultiplier),
            Some(specification)
                if !specification.contract_multiplier.is_finite()
                    || specification.contract_multiplier <= 0.0 =>
            {
                reasons.push(ExclusionReason::InvalidMultiplier)
            }
            Some(_) => {}
        }
        if !contract.strike.is_finite() {
            reasons.push(ExclusionReason::InvalidStrike);
        }
        if !reasons.is_empty() {
            for reason in &reasons {
                *excluded_by_reason.entry(*reason).or_insert(0) += 1;
            }
            if samples.len() < EXCLUSION_SAMPLE_LIMIT {
                samples.push(ExclusionSample {
                    occ_symbol: contract.occ_symbol.clone(),
                    reasons,
                });
            }
            continue;
        }

        let (Some(gamma), Some(open_interest), Some(specification), Some(spot)) = (
            contract.gamma,
            contract.open_interest,
            contract.contract_specification.as_ref(),
            spot,
        ) else {
            continue;
        };
        let Some(unsigned_gex) = checked_gex(
            gamma,
            open_interest,
            specification.contract_multiplier,
            spot,
        ) else {
            record_exclusion(
                contract.occ_symbol.clone(),
                vec![ExclusionReason::NumericOverflow],
                &mut excluded_by_reason,
                &mut samples,
            );
            continue;
        };
        let strike_key = contract.strike.to_bits();
        let strike_totals = strikes
            .get(&strike_key)
            .map_or(Totals::default(), |(_, totals)| *totals);
        let expiration_totals = expirations
            .get(&contract.expiration)
            .copied()
            .unwrap_or_default();
        let (Some(next_totals), Some(next_strike), Some(next_expiration)) = (
            totals.added(contract.option_type, unsigned_gex),
            strike_totals.added(contract.option_type, unsigned_gex),
            expiration_totals.added(contract.option_type, unsigned_gex),
        ) else {
            record_exclusion(
                contract.occ_symbol.clone(),
                vec![ExclusionReason::NumericOverflow],
                &mut excluded_by_reason,
                &mut samples,
            );
            continue;
        };
        included += 1;
        totals = next_totals;
        strikes.insert(strike_key, (contract.strike, next_strike));
        expirations.insert(contract.expiration, next_expiration);
    }

    let total_contracts = snapshot.contratos.len() as u64;
    GammaExposure {
        ticker: snapshot.ticker.clone(),
        spot,
        currency: snapshot
            .underlying_price
            .as_ref()
            .and_then(|price| price.currency.clone()),
        as_of: factual_as_of(snapshot, origin),
        snapshot_origin: origin,
        calls_gex: totals.calls,
        puts_gex: totals.puts,
        net_gex: totals.net(),
        by_strike: {
            let mut buckets = strikes
                .into_values()
                .map(|(key, totals)| GammaExposureBucket {
                    key,
                    calls_gex: totals.calls,
                    puts_gex: totals.puts,
                    net_gex: totals.net(),
                })
                .collect::<Vec<_>>();
            buckets.sort_by(|left, right| left.key.total_cmp(&right.key));
            buckets
        },
        by_expiration: expirations
            .into_iter()
            .map(|(key, totals)| GammaExposureBucket {
                key,
                calls_gex: totals.calls,
                puts_gex: totals.puts,
                net_gex: totals.net(),
            })
            .collect(),
        methodology: METHODOLOGY,
        sign_convention: SIGN_CONVENTION,
        diagnostics: GammaExposureDiagnostics {
            total_contracts,
            included_contracts: included,
            excluded_contracts: total_contracts - included,
            excluded_by_reason,
            samples,
            sample_limit: EXCLUSION_SAMPLE_LIMIT,
        },
    }
}

fn factual_as_of(snapshot: &Snapshot, origin: SnapshotOrigin) -> Option<DateTime<Utc>> {
    snapshot
        .underlying_price
        .as_ref()
        .and_then(|price| price.observed_at)
        .or_else(|| match snapshot.provider_timestamp.as_ref() {
            Some(provider) if provider.timezone == ProviderTimestampTimezone::VerifiedOffset => {
                Some(snapshot.timestamp_utc)
            }
            Some(_) => None,
            None if origin == SnapshotOrigin::EndOfDay => Some(snapshot.timestamp_utc),
            None => None,
        })
}

fn checked_gex(gamma: f64, open_interest: f64, multiplier: f64, spot: f64) -> Option<f64> {
    [gamma, 0.01, open_interest, multiplier, spot, spot]
        .into_iter()
        .try_fold(1.0, |value, factor| {
            let product = value * factor;
            product.is_finite().then_some(product)
        })
}

fn record_exclusion(
    occ_symbol: String,
    reasons: Vec<ExclusionReason>,
    excluded_by_reason: &mut BTreeMap<ExclusionReason, u64>,
    samples: &mut Vec<ExclusionSample>,
) {
    for reason in &reasons {
        *excluded_by_reason.entry(*reason).or_insert(0) += 1;
    }
    if samples.len() < EXCLUSION_SAMPLE_LIMIT {
        samples.push(ExclusionSample {
            occ_symbol,
            reasons,
        });
    }
}
