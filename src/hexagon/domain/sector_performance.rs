use chrono::{DateTime, NaiveDate, Utc};

pub const SECTOR_BENCHMARK_TICKER: &str = "SPY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sector {
    pub name: &'static str,
    pub etf: &'static str,
}

pub const SECTORS: [Sector; 11] = [
    Sector {
        name: "Tecnologia",
        etf: "XLK",
    },
    Sector {
        name: "Financeiro",
        etf: "XLF",
    },
    Sector {
        name: "Energia",
        etf: "XLE",
    },
    Sector {
        name: "Industrial",
        etf: "XLI",
    },
    Sector {
        name: "Saúde",
        etf: "XLV",
    },
    Sector {
        name: "Consumo discricionário",
        etf: "XLY",
    },
    Sector {
        name: "Comunicações",
        etf: "XLC",
    },
    Sector {
        name: "Bens essenciais",
        etf: "XLP",
    },
    Sector {
        name: "Materiais",
        etf: "XLB",
    },
    Sector {
        name: "Utilities",
        etf: "XLU",
    },
    Sector {
        name: "Imobiliário",
        etf: "XLRE",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorPerformancePeriod {
    OneWeek,
    TwoWeeks,
    OneMonth,
}

impl SectorPerformancePeriod {
    pub const fn sessions(self) -> usize {
        match self {
            Self::OneWeek => 5,
            Self::TwoWeeks => 10,
            Self::OneMonth => 21,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentPerformance {
    pub ticker: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub return_percent: f64,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceState<T> {
    Available(T),
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectorComparison {
    pub sector: Sector,
    pub performance: InstrumentPerformance,
    pub relative_strength_percentage_points: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectorPerformanceItem {
    pub sector: Sector,
    pub comparison: PerformanceState<SectorComparison>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectorPerformanceView {
    pub as_of: NaiveDate,
    pub period: SectorPerformancePeriod,
    pub benchmark: PerformanceState<InstrumentPerformance>,
    pub sectors: Vec<SectorPerformanceItem>,
}

pub fn percentage_return(start: f64, end: f64) -> Option<f64> {
    if !start.is_finite() || !end.is_finite() || start <= 0.0 || end <= 0.0 {
        return None;
    }
    Some((end / start - 1.0) * 100.0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn catalog_has_exactly_the_eleven_sp_500_sector_etfs_without_duplicates() {
        assert_eq!(SECTORS.len(), 11);
        assert_eq!(
            SECTORS.map(|sector| sector.etf),
            [
                "XLK", "XLF", "XLE", "XLI", "XLV", "XLY", "XLC", "XLP", "XLB", "XLU", "XLRE",
            ]
        );
        assert_eq!(
            SECTORS
                .iter()
                .map(|sector| sector.etf)
                .collect::<HashSet<_>>()
                .len(),
            11
        );
    }

    #[test]
    fn periods_have_trading_session_semantics() {
        assert_eq!(SectorPerformancePeriod::OneWeek.sessions(), 5);
        assert_eq!(SectorPerformancePeriod::TwoWeeks.sessions(), 10);
        assert_eq!(SectorPerformancePeriod::OneMonth.sessions(), 21);
    }

    #[test]
    fn return_is_percentage_change_and_rejects_invalid_prices() {
        assert_eq!(percentage_return(100.0, 110.0), Some(10.000000000000009));
        for invalid in [(0.0, 10.0), (-1.0, 10.0), (10.0, 0.0), (f64::NAN, 10.0)] {
            assert_eq!(percentage_return(invalid.0, invalid.1), None);
        }
    }
}
