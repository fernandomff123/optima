use std::error::Error;

use chrono::NaiveDate;

use super::client::{TreasuryFeed, TreasuryProperties};
use crate::hexagon::domain::treasury::YieldCurve;

pub fn feed_to_yield_curves(feed: TreasuryFeed) -> Result<Vec<YieldCurve>, Box<dyn Error>> {
    let mut curves = Vec::with_capacity(feed.entries.len());

    for entry in feed.entries {
        curves.push(properties_to_yield_curve(entry.content.properties)?);
    }

    curves.sort_by_key(|curve| curve.date);
    Ok(curves)
}

fn properties_to_yield_curve(properties: TreasuryProperties) -> Result<YieldCurve, Box<dyn Error>> {
    let date = NaiveDate::parse_from_str(
        properties.date.get(..10).ok_or("data inválida")?,
        "%Y-%m-%d",
    )?;

    Ok(YieldCurve {
        date,
        m1: decimalize(properties.m1),
        m2: decimalize(properties.m2),
        m3: decimalize(properties.m3),
        m6: decimalize(properties.m6),
        y1: decimalize(properties.y1),
        y2: decimalize(properties.y2),
        y3: decimalize(properties.y3),
        y5: decimalize(properties.y5),
        y7: decimalize(properties.y7),
        y10: decimalize(properties.y10),
        y20: decimalize(properties.y20),
        y30: decimalize(properties.y30),
    })
}

fn decimalize(value: Option<f64>) -> Option<f64> {
    value.map(|rate| rate / 100.0)
}
