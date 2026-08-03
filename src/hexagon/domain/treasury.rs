use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq)]
pub struct YieldCurve {
    pub date: NaiveDate,
    pub m1: Option<f64>,
    pub m2: Option<f64>,
    pub m3: Option<f64>,
    pub m6: Option<f64>,
    pub y1: Option<f64>,
    pub y2: Option<f64>,
    pub y3: Option<f64>,
    pub y5: Option<f64>,
    pub y7: Option<f64>,
    pub y10: Option<f64>,
    pub y20: Option<f64>,
    pub y30: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreasuryRow {
    pub date: NaiveDate,
    pub rates: YieldCurve,
}
