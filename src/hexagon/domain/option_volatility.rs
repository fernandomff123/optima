use chrono::{Datelike, NaiveDate, Weekday};
use std::error::Error;
use std::io;

use crate::hexagon::domain::interest_rates::BoundedCubicSpline;
use crate::hexagon::domain::options::{ContratoOpcao, OptionType, Snapshot};
use crate::hexagon::domain::treasury::YieldCurve;
use crate::hexagon::domain::volatility::{
    ConstantMaturityVolatility, TermStructure, TermStructurePoint, TermStructureSource, Volatility,
};

pub const TERM_STRUCTURE_START_DAYS: f64 = 30.0;

struct StrikePair<'a> {
    strike: f64,
    call: &'a ContratoOpcao,
    put: &'a ContratoOpcao,
}

/// Calcula a volatilidade implícita agregada de uma maturidade.
///
/// `time_to_expiration` é fornecido pela aplicação em anos. Dessa forma, a
/// regra financeira permanece independente do calendário usado pelo mercado.
pub fn calculate_volatility(
    snapshot: &Snapshot,
    expiration: NaiveDate,
    interest_rate: f64,
    time_to_expiration: f64,
) -> Result<Volatility, Box<dyn Error + Send + Sync>> {
    calculate_volatility_from_contracts(
        &snapshot.contratos,
        expiration,
        interest_rate,
        time_to_expiration,
    )
}

fn calculate_volatility_from_contracts(
    contracts: &[ContratoOpcao],
    expiration: NaiveDate,
    interest_rate: f64,
    time_to_expiration: f64,
) -> Result<Volatility, Box<dyn Error + Send + Sync>> {
    if !time_to_expiration.is_finite() || time_to_expiration <= 0.0001 {
        return Err(invalid_data("a maturidade já expirou"));
    }

    let pairs = pair_contracts(contracts, expiration);
    if pairs.is_empty() {
        return Err(invalid_data(
            "não existem calls e puts emparelhadas por strike",
        ));
    }

    let forward_pair = pairs
        .iter()
        .filter(|pair| pair.call.bid > 0.0 && pair.put.bid > 0.0)
        .min_by(|left, right| {
            let left_difference = (left.call.mid - left.put.mid).abs();
            let right_difference = (right.call.mid - right.put.mid).abs();
            left_difference
                .total_cmp(&right_difference)
                .then_with(|| left.strike.total_cmp(&right.strike))
        })
        .ok_or_else(|| invalid_data("não foi possível determinar o strike ATM com bids válidos"))?;
    let discount_factor = (interest_rate * time_to_expiration).exp();
    let forward =
        forward_pair.strike + discount_factor * (forward_pair.call.mid - forward_pair.put.mid);

    let k0_pair = pairs
        .iter()
        .filter(|pair| pair.strike <= forward)
        .max_by(|left, right| left.strike.total_cmp(&right.strike))
        .ok_or_else(|| invalid_data("não existe um strike igual ou inferior ao forward"))?;
    let k0 = k0_pair.strike;
    let mut contributions = selected_otm_options(&pairs, k0);
    contributions.push((k0, (k0_pair.call.mid + k0_pair.put.mid) / 2.0));
    contributions.sort_by(|left, right| left.0.total_cmp(&right.0));

    if contributions.len() < 2 {
        return Err(invalid_data("não existem opções OTM suficientes"));
    }

    let mut sum = 0.0;
    for index in 0..contributions.len() {
        let (strike, option_mid) = contributions[index];
        let delta_k = if index == 0 {
            contributions[1].0 - strike
        } else if index == contributions.len() - 1 {
            strike - contributions[index - 1].0
        } else {
            (contributions[index + 1].0 - contributions[index - 1].0) / 2.0
        };
        sum += delta_k / strike.powi(2) * discount_factor * option_mid;
    }

    let variance =
        2.0 / time_to_expiration * sum - (forward / k0 - 1.0).powi(2) / time_to_expiration;
    if variance < 0.0 {
        return Err(invalid_data("a variância calculada é negativa"));
    }

    Ok(Volatility {
        expiration,
        time_to_expiration,
        forward,
        k0,
        variance,
        volatility: variance.sqrt() * 100.0,
    })
}

/// Interpola a variância total de duas maturidades para um prazo constante.
///
/// A fórmula combina a variância total das opções near-term e next-term.
/// As duas volatilidades devem envolver o prazo-alvo.
pub fn interpolate_constant_maturity(
    near: &Volatility,
    next: &Volatility,
    target_days: f64,
) -> Result<ConstantMaturityVolatility, Box<dyn Error + Send + Sync>> {
    if !target_days.is_finite() || target_days <= 0.0 {
        return Err(invalid_data("o prazo-alvo deve ser positivo"));
    }
    if near.time_to_expiration >= next.time_to_expiration {
        return Err(invalid_data(
            "a maturidade near deve ser anterior à maturidade next",
        ));
    }

    let target_time = target_days / 365.0;
    if target_time < near.time_to_expiration || target_time > next.time_to_expiration {
        return Err(invalid_data(
            "as maturidades não envolvem o prazo-alvo solicitado",
        ));
    }

    let interval = next.time_to_expiration - near.time_to_expiration;
    let near_weight = (next.time_to_expiration - target_time) / interval;
    let next_weight = (target_time - near.time_to_expiration) / interval;
    let target_total_variance = near.time_to_expiration * near.variance * near_weight
        + next.time_to_expiration * next.variance * next_weight;
    let variance = target_total_variance / target_time;
    if variance < 0.0 {
        return Err(invalid_data("a variância interpolada é negativa"));
    }

    Ok(ConstantMaturityVolatility {
        target_days,
        near_expiration: near.expiration,
        next_expiration: next.expiration,
        variance,
        volatility: variance.sqrt() * 100.0,
    })
}

/// Constrói a term structure a partir dos 30 dias.
///
/// O primeiro ponto é interpolado entre as maturidades de sexta-feira
/// imediatamente abaixo e acima de 30 dias. Quando uma série semanal terminada
/// em `W` e a respetiva série principal expiram na mesma data, é usada a série
/// principal. Os restantes pontos representam cada expiração válida posterior,
/// usando a taxa Treasury correspondente ao seu prazo.
pub fn calculate_term_structure<F>(
    snapshot: &Snapshot,
    treasury_curve: &YieldCurve,
    mut time_to_expiration: F,
) -> Result<TermStructure, Box<dyn Error + Send + Sync>>
where
    F: FnMut(NaiveDate, bool) -> Result<f64, Box<dyn Error + Send + Sync>>,
{
    if treasury_curve.date > snapshot.timestamp_utc.date_naive() {
        return Err(invalid_data(
            "a curva Treasury não pode ser posterior ao snapshot",
        ));
    }

    let spline = BoundedCubicSpline::from_treasury_curve(treasury_curve)?;
    let mut maturities = Vec::new();
    let chains: Vec<_> = if snapshot.chains.is_empty() {
        vec![(snapshot.ticker.as_str(), snapshot.contratos.as_slice())]
    } else {
        snapshot
            .chains
            .iter()
            .map(|chain| (chain.root.as_str(), chain.contratos.as_slice()))
            .collect()
    };
    for (root, contracts) in chains {
        if is_adjusted_root(root) {
            continue;
        }
        let mut expirations: Vec<_> = contracts
            .iter()
            .map(|contract| contract.expiration)
            .collect();
        expirations.sort_unstable();
        expirations.dedup();
        let is_pm = is_pm_settlement(root);

        for expiration in expirations {
            let Ok(expiration_time) = time_to_expiration(expiration, is_pm) else {
                continue;
            };
            let rate_days = expiration
                .signed_duration_since(treasury_curve.date)
                .num_days() as f64;
            let Ok(interest_rate) = spline.continuously_compounded_rate(rate_days) else {
                continue;
            };
            let Ok(volatility) = calculate_volatility_from_contracts(
                contracts,
                expiration,
                interest_rate,
                expiration_time,
            ) else {
                continue;
            };
            maturities.push((volatility, interest_rate, root.to_string()));
        }
    }
    maturities.sort_by(|left, right| {
        left.0
            .time_to_expiration
            .total_cmp(&right.0.time_to_expiration)
    });

    let target_time = TERM_STRUCTURE_START_DAYS / 365.0;
    let (near, near_rate, _) = maturities
        .iter()
        .rev()
        .find(|(result, _, root)| {
            is_eligible_constant_maturity(result, root, &maturities)
                && result.time_to_expiration < target_time
        })
        .ok_or_else(|| invalid_data("não existe uma maturidade válida abaixo de 30 dias"))?;
    let (next, next_rate, _) = maturities
        .iter()
        .find(|(result, _, root)| {
            is_eligible_constant_maturity(result, root, &maturities)
                && result.time_to_expiration > target_time
        })
        .ok_or_else(|| invalid_data("não existe uma maturidade válida acima de 30 dias"))?;
    let interpolated = interpolate_constant_maturity(near, next, TERM_STRUCTURE_START_DAYS)?;

    let mut points = Vec::with_capacity(maturities.len());
    points.push(TermStructurePoint {
        days: TERM_STRUCTURE_START_DAYS,
        variance: interpolated.variance,
        volatility: interpolated.volatility,
        source: TermStructureSource::Interpolated {
            near_expiration: near.expiration,
            near_rate: *near_rate,
            next_expiration: next.expiration,
            next_rate: *next_rate,
        },
    });
    points.extend(
        maturities
            .into_iter()
            .filter(|(result, _, _)| result.time_to_expiration > target_time)
            .map(|(result, interest_rate, _)| TermStructurePoint {
                days: result.time_to_expiration * 365.0,
                variance: result.variance,
                volatility: result.volatility,
                source: TermStructureSource::Expiration {
                    expiration: result.expiration,
                    interest_rate,
                },
            }),
    );

    Ok(TermStructure {
        ticker: snapshot.ticker.clone(),
        snapshot_timestamp: snapshot.timestamp_utc,
        treasury_date: treasury_curve.date,
        points,
    })
}

fn is_adjusted_root(root: &str) -> bool {
    root.chars()
        .last()
        .is_some_and(|character| character.is_ascii_digit())
}

fn is_eligible_constant_maturity(
    result: &Volatility,
    root: &str,
    maturities: &[(Volatility, f64, String)],
) -> bool {
    if result.expiration.weekday() != Weekday::Fri {
        return false;
    }
    let Some(primary_root) = root
        .strip_suffix('W')
        .or_else(|| root.strip_suffix('w'))
        .filter(|primary| !primary.is_empty())
    else {
        return true;
    };
    !maturities.iter().any(|(other, _, other_root)| {
        other_root.eq_ignore_ascii_case(primary_root) && other.expiration == result.expiration
    })
}

fn is_pm_settlement(root: &str) -> bool {
    !root.eq_ignore_ascii_case("SPX")
}

fn pair_contracts(contracts: &[ContratoOpcao], expiration: NaiveDate) -> Vec<StrikePair<'_>> {
    let mut calls: Vec<_> = contracts
        .iter()
        .filter(|contract| {
            contract.expiration == expiration && contract.option_type == OptionType::Call
        })
        .collect();
    let mut puts: Vec<_> = contracts
        .iter()
        .filter(|contract| {
            contract.expiration == expiration && contract.option_type == OptionType::Put
        })
        .collect();
    calls.sort_by(|left, right| left.strike.total_cmp(&right.strike));
    puts.sort_by(|left, right| left.strike.total_cmp(&right.strike));

    let (mut call_index, mut put_index) = (0, 0);
    let mut pairs = Vec::new();
    while call_index < calls.len() && put_index < puts.len() {
        match calls[call_index].strike.total_cmp(&puts[put_index].strike) {
            std::cmp::Ordering::Less => call_index += 1,
            std::cmp::Ordering::Greater => put_index += 1,
            std::cmp::Ordering::Equal => {
                pairs.push(StrikePair {
                    strike: calls[call_index].strike,
                    call: calls[call_index],
                    put: puts[put_index],
                });
                call_index += 1;
                put_index += 1;
            }
        }
    }
    pairs
}

fn selected_otm_options(pairs: &[StrikePair<'_>], k0: f64) -> Vec<(f64, f64)> {
    let mut selected = Vec::new();

    let mut zero_bids = 0;
    for pair in pairs.iter().filter(|pair| pair.strike < k0).rev() {
        if pair.put.bid <= 0.0 {
            zero_bids += 1;
            if zero_bids == 2 {
                break;
            }
        } else {
            zero_bids = 0;
            selected.push((pair.strike, pair.put.mid));
        }
    }

    zero_bids = 0;
    for pair in pairs.iter().filter(|pair| pair.strike > k0) {
        if pair.call.bid <= 0.0 {
            zero_bids += 1;
            if zero_bids == 2 {
                break;
            }
        } else {
            zero_bids = 0;
            selected.push((pair.strike, pair.call.mid));
        }
    }

    selected
}

fn invalid_data(message: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn distinguishes_standard_spx_am_from_spxw_pm_settlement() {
        assert!(!is_pm_settlement("SPX"));
        assert!(is_pm_settlement("SPXW"));
        assert!(is_pm_settlement("SPY"));
    }

    #[test]
    fn identifies_roots_ending_in_digits_as_adjusted() {
        assert!(is_adjusted_root("IBM1"));
        assert!(is_adjusted_root("AAPL12"));
        assert!(!is_adjusted_root("IBM"));
        assert!(!is_adjusted_root("SPXW"));
        assert!(!is_adjusted_root(""));
    }

    #[test]
    fn prefers_the_primary_root_over_a_variant_on_the_same_friday() {
        let monthly_expiration = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
        let weekly_expiration = NaiveDate::from_ymd_opt(2026, 8, 28).unwrap();
        let volatility = |expiration| Volatility {
            expiration,
            time_to_expiration: 28.0 / 365.0,
            forward: 100.0,
            k0: 100.0,
            variance: 0.04,
            volatility: 20.0,
        };
        let maturities = vec![
            (volatility(monthly_expiration), 0.04, "SPX".to_string()),
            (volatility(monthly_expiration), 0.04, "SPXW".to_string()),
            (volatility(weekly_expiration), 0.04, "SPXW".to_string()),
        ];

        assert!(is_eligible_constant_maturity(
            &maturities[0].0,
            &maturities[0].2,
            &maturities
        ));
        assert!(!is_eligible_constant_maturity(
            &maturities[1].0,
            &maturities[1].2,
            &maturities
        ));
        assert!(is_eligible_constant_maturity(
            &maturities[2].0,
            &maturities[2].2,
            &maturities
        ));
    }

    fn contract(
        option_type: OptionType,
        strike: f64,
        expiration: NaiveDate,
        bid: f64,
        mid: f64,
    ) -> ContratoOpcao {
        ContratoOpcao {
            occ_symbol: format!("TEST-{strike}"),
            option_type,
            strike,
            expiration,
            bid,
            ask: 2.0 * mid - bid,
            mid,
            spread: 2.0 * (mid - bid),
            volume: 0.0,
            open_interest: 0.0,
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            theo: 0.0,
            implied_volatility: None,
        }
    }

    #[test]
    fn calculates_forward_and_volatility_from_domain_contracts() {
        let expiration = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
        let quotes = [
            (90.0, 12.0, 1.0, 11.5, 0.5),
            (95.0, 8.0, 2.0, 7.5, 1.5),
            (100.0, 6.0, 4.0, 5.5, 3.5),
            (105.0, 3.0, 7.0, 2.5, 6.5),
            (110.0, 1.0, 11.0, 0.5, 10.5),
        ];
        let mut contracts = Vec::new();
        for (strike, call_mid, put_mid, call_bid, put_bid) in quotes {
            contracts.push(contract(
                OptionType::Call,
                strike,
                expiration,
                call_bid,
                call_mid,
            ));
            contracts.push(contract(
                OptionType::Put,
                strike,
                expiration,
                put_bid,
                put_mid,
            ));
        }
        contracts.push(contract(OptionType::Call, 50.0, expiration, 0.0, 0.0));
        contracts.push(contract(OptionType::Put, 50.0, expiration, 0.0, 0.0));
        let snapshot = Snapshot {
            ticker: "TEST".to_string(),
            timestamp_utc: NaiveDate::from_ymd_opt(2026, 7, 13)
                .unwrap()
                .and_hms_opt(15, 0, 0)
                .unwrap()
                .and_utc(),
            contratos: contracts,
            chains: Vec::new(),
        };

        let result = calculate_volatility(&snapshot, expiration, 0.0, 39.0 / 365.0).unwrap();

        assert_eq!(result.forward, 102.0);
        assert_eq!(result.k0, 100.0);
        assert!(result.variance > 0.0);
        assert!(result.volatility > 0.0);
        assert!((result.volatility - result.variance.sqrt() * 100.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_an_expiration_without_paired_contracts() {
        let snapshot = Snapshot {
            ticker: "TEST".to_string(),
            timestamp_utc: Utc::now(),
            contratos: Vec::new(),
            chains: Vec::new(),
        };
        let expiration = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();

        assert!(calculate_volatility(&snapshot, expiration, 0.0, 39.0 / 365.0).is_err());
    }

    #[test]
    fn stops_each_otm_wing_after_two_consecutive_zero_bids() {
        let expiration = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
        let quotes = [
            (80.0, 1.0, 1.0),
            (85.0, 1.0, 0.0),
            (90.0, 1.0, 0.0),
            (95.0, 1.0, 1.0),
            (100.0, 1.0, 1.0),
            (105.0, 1.0, 1.0),
            (110.0, 0.0, 1.0),
            (115.0, 1.0, 1.0),
            (120.0, 0.0, 1.0),
            (125.0, 0.0, 1.0),
            (130.0, 1.0, 1.0),
        ];
        let mut contracts = Vec::new();
        for (strike, call_bid, put_bid) in quotes {
            contracts.push(contract(
                OptionType::Call,
                strike,
                expiration,
                call_bid,
                1.0,
            ));
            contracts.push(contract(OptionType::Put, strike, expiration, put_bid, 1.0));
        }

        let pairs = pair_contracts(&contracts, expiration);
        let selected = selected_otm_options(&pairs, 100.0);
        let strikes: Vec<_> = selected.into_iter().map(|(strike, _)| strike).collect();

        assert_eq!(strikes, vec![95.0, 105.0, 115.0]);
    }

    #[test]
    fn interpolates_total_variance_between_two_expirations() {
        let near = Volatility {
            expiration: NaiveDate::from_ymd_opt(2026, 8, 7).unwrap(),
            time_to_expiration: 20.0 / 365.0,
            forward: 100.0,
            k0: 100.0,
            variance: 0.04,
            volatility: 20.0,
        };
        let next = Volatility {
            expiration: NaiveDate::from_ymd_opt(2026, 8, 27).unwrap(),
            time_to_expiration: 40.0 / 365.0,
            forward: 101.0,
            k0: 100.0,
            variance: 0.09,
            volatility: 30.0,
        };

        let result = interpolate_constant_maturity(&near, &next, 30.0).unwrap();

        let expected_variance =
            ((20.0 / 365.0) * 0.04 * 0.5 + (40.0 / 365.0) * 0.09 * 0.5) / (30.0 / 365.0);
        assert!((result.variance - expected_variance).abs() < 1e-12);
        assert!((result.volatility - expected_variance.sqrt() * 100.0).abs() < 1e-12);
        assert_eq!(result.near_expiration, near.expiration);
        assert_eq!(result.next_expiration, next.expiration);
    }

    #[test]
    fn rejects_expirations_that_do_not_bracket_target() {
        let near = Volatility {
            expiration: NaiveDate::from_ymd_opt(2026, 8, 7).unwrap(),
            time_to_expiration: 10.0 / 365.0,
            forward: 100.0,
            k0: 100.0,
            variance: 0.04,
            volatility: 20.0,
        };
        let next = Volatility {
            expiration: NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
            time_to_expiration: 20.0 / 365.0,
            forward: 100.0,
            k0: 100.0,
            variance: 0.05,
            volatility: 0.05_f64.sqrt() * 100.0,
        };

        assert!(interpolate_constant_maturity(&near, &next, 30.0).is_err());
    }

    #[test]
    fn builds_term_structure_starting_with_interpolated_30_days() {
        let snapshot_date = NaiveDate::from_ymd_opt(2026, 7, 13).unwrap();
        let expirations = [
            NaiveDate::from_ymd_opt(2026, 8, 7).unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 11).unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 13).unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
        ];
        let mut contracts = Vec::new();
        for expiration in expirations {
            for (strike, call_mid, put_mid, call_bid, put_bid) in [
                (95.0, 8.0, 2.0, 7.5, 1.5),
                (100.0, 6.0, 4.0, 5.5, 3.5),
                (105.0, 3.0, 7.0, 2.5, 6.5),
            ] {
                contracts.push(contract(
                    OptionType::Call,
                    strike,
                    expiration,
                    call_bid,
                    call_mid,
                ));
                contracts.push(contract(
                    OptionType::Put,
                    strike,
                    expiration,
                    put_bid,
                    put_mid,
                ));
            }
        }
        let snapshot = Snapshot {
            ticker: "TEST".to_string(),
            timestamp_utc: snapshot_date.and_hms_opt(15, 0, 0).unwrap().and_utc(),
            contratos: contracts,
            chains: Vec::new(),
        };
        let treasury = YieldCurve {
            date: snapshot_date,
            m1: Some(0.04),
            m2: Some(0.041),
            m3: Some(0.042),
            m6: Some(0.043),
            y1: Some(0.044),
            y2: Some(0.045),
            y3: Some(0.046),
            y5: Some(0.047),
            y7: Some(0.048),
            y10: Some(0.049),
            y20: Some(0.05),
            y30: Some(0.051),
        };

        let term_structure = calculate_term_structure(&snapshot, &treasury, |expiration, _| {
            Ok(expiration.signed_duration_since(snapshot_date).num_days() as f64 / 365.0)
        })
        .unwrap();

        assert_eq!(term_structure.ticker, "TEST");
        assert_eq!(term_structure.treasury_date, snapshot_date);
        assert_eq!(term_structure.points[0].days, 30.0);
        assert!(matches!(
            term_structure.points[0].source,
            TermStructureSource::Interpolated {
                near_expiration,
                next_expiration,
                ..
            } if near_expiration == expirations[0] && next_expiration == expirations[3]
        ));
        assert!(term_structure.points.iter().all(|point| point.days >= 30.0));
        assert!(
            term_structure
                .points
                .windows(2)
                .all(|points| points[0].days < points[1].days)
        );
    }
}
