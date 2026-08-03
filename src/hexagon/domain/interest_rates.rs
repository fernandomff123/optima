use std::error::Error;
use std::fmt;

use crate::hexagon::domain::treasury::YieldCurve;

#[derive(Debug, Clone, PartialEq)]
pub enum InterestRateError {
    InsufficientPoints,
    InvalidPoint,
    DuplicateMaturity,
    InvalidTarget,
    InvalidBondEquivalentYield,
}

impl fmt::Display for InterestRateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InsufficientPoints => "são necessários pelo menos dois pontos da curva",
            Self::InvalidPoint => "a curva contém um prazo ou uma taxa inválida",
            Self::DuplicateMaturity => "a curva contém prazos repetidos",
            Self::InvalidTarget => "o prazo-alvo é inválido",
            Self::InvalidBondEquivalentYield => "a taxa BEY não permite conversão para APY",
        };
        formatter.write_str(message)
    }
}

impl Error for InterestRateError {}

/// Spline cúbica natural limitada para interpolar uma curva de taxas.
///
/// Os prazos são expressos em dias e as taxas BEY em formato decimal. A
/// implementação pertence ao domínio: não conhece fontes de dados, bases de
/// dados nem bibliotecas de calendário.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundedCubicSpline {
    days: Vec<f64>,
    rates: Vec<f64>,
    b: Vec<f64>,
    c: Vec<f64>,
    d: Vec<f64>,
}

impl BoundedCubicSpline {
    pub fn from_treasury_curve(curve: &YieldCurve) -> Result<Self, InterestRateError> {
        let candidates = [
            (30.0, curve.m1),
            (60.0, curve.m2),
            (91.0, curve.m3),
            (182.0, curve.m6),
            (365.0, curve.y1),
            (730.0, curve.y2),
            (1_095.0, curve.y3),
            (1_825.0, curve.y5),
            (2_555.0, curve.y7),
            (3_650.0, curve.y10),
            (7_300.0, curve.y20),
            (10_950.0, curve.y30),
        ];
        let points: Vec<_> = candidates
            .into_iter()
            .filter_map(|(days, rate)| rate.map(|rate| (days, rate)))
            .collect();
        Self::new(&points)
    }

    pub fn new(points: &[(f64, f64)]) -> Result<Self, InterestRateError> {
        if points.len() < 2 {
            return Err(InterestRateError::InsufficientPoints);
        }
        let mut points = points.to_vec();
        if points
            .iter()
            .any(|(days, rate)| !days.is_finite() || *days <= 0.0 || !rate.is_finite())
        {
            return Err(InterestRateError::InvalidPoint);
        }
        points.sort_by(|left, right| left.0.total_cmp(&right.0));
        if points.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(InterestRateError::DuplicateMaturity);
        }

        let days: Vec<_> = points.iter().map(|point| point.0).collect();
        let rates: Vec<_> = points.iter().map(|point| point.1).collect();
        let (b, c, d) = natural_spline_coefficients(&days, &rates);
        Ok(Self {
            days,
            rates,
            b,
            c,
            d,
        })
    }

    pub fn bond_equivalent_yield(&self, target_days: f64) -> Result<f64, InterestRateError> {
        if !target_days.is_finite() || target_days < 0.0 {
            return Err(InterestRateError::InvalidTarget);
        }

        if let Ok(index) = self
            .days
            .binary_search_by(|days| days.total_cmp(&target_days))
        {
            return Ok(self.rates[index]);
        }
        if target_days < self.days[0] {
            let raw = self.evaluate_interval(0, target_days);
            let (lower, upper) = self.short_end_bounds(target_days);
            return Ok(raw.clamp(lower, upper));
        }
        let last = self.days.len() - 1;
        if target_days > self.days[last] {
            return Ok(self.rates[last]);
        }

        let upper = self.days.partition_point(|days| *days < target_days);
        let lower = upper - 1;
        let raw = self.evaluate_interval(lower, target_days);
        let lower_bound = self.rates[lower].min(self.rates[upper]);
        let upper_bound = self.rates[lower].max(self.rates[upper]);
        Ok(raw.clamp(lower_bound, upper_bound))
    }

    pub fn continuously_compounded_rate(&self, target_days: f64) -> Result<f64, InterestRateError> {
        let bey = self.bond_equivalent_yield(target_days)?;
        if bey <= -2.0 {
            return Err(InterestRateError::InvalidBondEquivalentYield);
        }
        // APY = (1 + BEY / 2)^2 - 1; r = ln(1 + APY).
        Ok(2.0 * (1.0 + bey / 2.0).ln())
    }

    fn evaluate_interval(&self, interval: usize, target_days: f64) -> f64 {
        let dx = target_days - self.days[interval];
        self.rates[interval]
            + self.b[interval] * dx
            + self.c[interval] * dx.powi(2)
            + self.d[interval] * dx.powi(3)
    }

    fn short_end_bounds(&self, target_days: f64) -> (f64, f64) {
        let first_day = self.days[0];
        let first_rate = self.rates[0];
        let lower_slope = self
            .rates
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, rate)| **rate >= first_rate)
            .map(|(index, rate)| (rate - first_rate) / (self.days[index] - first_day))
            .unwrap_or(0.0);
        let upper_slope = self
            .rates
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, rate)| **rate <= first_rate)
            .map(|(index, rate)| (rate - first_rate) / (self.days[index] - first_day))
            .unwrap_or(0.0);
        let first_offset = target_days - first_day;
        let lower = first_rate + lower_slope * first_offset;
        let upper = first_rate + upper_slope * first_offset;
        (lower.min(upper), lower.max(upper))
    }
}

fn natural_spline_coefficients(days: &[f64], rates: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let count = days.len();
    let intervals = count - 1;
    let widths: Vec<_> = days.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let mut b = vec![0.0; intervals];
    let mut c = vec![0.0; count];
    let mut d = vec![0.0; intervals];

    if count == 2 {
        b[0] = (rates[1] - rates[0]) / widths[0];
        return (b, c, d);
    }

    let mut alpha = vec![0.0; count];
    for index in 1..intervals {
        alpha[index] = 3.0 / widths[index] * (rates[index + 1] - rates[index])
            - 3.0 / widths[index - 1] * (rates[index] - rates[index - 1]);
    }
    let mut diagonal = vec![1.0; count];
    let mut upper = vec![0.0; count];
    let mut solution = vec![0.0; count];
    for index in 1..intervals {
        diagonal[index] =
            2.0 * (days[index + 1] - days[index - 1]) - widths[index - 1] * upper[index - 1];
        upper[index] = widths[index] / diagonal[index];
        solution[index] =
            (alpha[index] - widths[index - 1] * solution[index - 1]) / diagonal[index];
    }
    for index in (0..intervals).rev() {
        c[index] = solution[index] - upper[index] * c[index + 1];
        b[index] = (rates[index + 1] - rates[index]) / widths[index]
            - widths[index] * (c[index + 1] + 2.0 * c[index]) / 3.0;
        d[index] = (c[index + 1] - c[index]) / (3.0 * widths[index]);
    }
    (b, c, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_knots_and_a_known_natural_spline_value() {
        let spline = BoundedCubicSpline::new(&[(1.0, 0.0), (2.0, 1.0), (3.0, 0.0)]).unwrap();

        assert_eq!(spline.bond_equivalent_yield(1.0).unwrap(), 0.0);
        assert_eq!(spline.bond_equivalent_yield(2.0).unwrap(), 1.0);
        assert_eq!(spline.bond_equivalent_yield(3.0).unwrap(), 0.0);
        assert!((spline.bond_equivalent_yield(1.5).unwrap() - 0.6875).abs() < 1e-12);
    }

    #[test]
    fn bounds_internal_spline_overshoot_by_adjacent_rates() {
        let spline =
            BoundedCubicSpline::new(&[(1.0, 0.0), (2.0, 1.0), (3.0, 1.0), (4.0, 0.0)]).unwrap();

        assert_eq!(spline.bond_equivalent_yield(2.5).unwrap(), 1.0);
    }

    #[test]
    fn applies_short_end_bounds() {
        let spline = BoundedCubicSpline::new(&[(30.0, 0.05), (60.0, 0.06), (91.0, 0.04)]).unwrap();

        let rate = spline.bond_equivalent_yield(20.0).unwrap();
        let lower = 0.05 + (0.06 - 0.05) / 30.0 * (20.0 - 30.0);
        let upper = 0.05 + (0.04 - 0.05) / 61.0 * (20.0 - 30.0);
        assert!(rate >= lower);
        assert!(rate <= upper);
    }

    #[test]
    fn converts_bey_to_continuously_compounded_rate() {
        let spline = BoundedCubicSpline::new(&[(30.0, 0.05), (60.0, 0.06)]).unwrap();
        let expected = 2.0 * 1.025_f64.ln();

        assert!((spline.continuously_compounded_rate(30.0).unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn reproduces_a_published_interest_rate_example() {
        let points = [
            (30.0, 0.0003),
            (60.0, 0.0002),
            (91.0, 0.0004),
            (182.0, 0.0005),
            (365.0, 0.0008),
            (730.0, 0.0011),
            (1_095.0, 0.0022),
            (1_825.0, 0.0059),
            (2_555.0, 0.0100),
            (3_650.0, 0.0137),
            (7_300.0, 0.0203),
            (10_950.0, 0.0221),
        ];
        let spline = BoundedCubicSpline::new(&points).unwrap();

        let near = spline.continuously_compounded_rate(25.0).unwrap();
        let next = spline.continuously_compounded_rate(32.0).unwrap();

        assert!((near - 0.00031664).abs() < 5e-8, "near={near}");
        assert!((next - 0.00028797).abs() < 5e-8, "next={next}");
    }
}
