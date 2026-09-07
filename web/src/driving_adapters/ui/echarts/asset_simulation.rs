use crate::{
    application::asset_simulation::{AssetSimulationReadModel, SimulationMetric},
    design_system::tokens,
    driving_adapters::ui::{components::ScenarioSelection, simulation_draft::DraftLeg},
    ports::asset_simulation::{
        MetricSentiment, StrategySimulationCurve, StrategySimulationGreeks,
        StrategySimulationResult,
    },
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-simulation-payoff-echarts";
const SELECTED_TIME_SERIES: &str = "Selected time";
const EXPIRATION_SERIES: &str = "At first expiration";

#[component]
pub fn SimulationPayoffChart(
    model: AssetSimulationReadModel,
    selection: RwSignal<ScenarioSelection>,
    draft_rows: RwSignal<Vec<DraftLeg>>,
    pricing: ReadSignal<Option<StrategySimulationResult>>,
) -> impl IntoView {
    let render_model = model.clone();
    Effect::new(move |_| {
        let draft = draft_rows.get();
        render_chart(
            HOST_ID,
            &build_payoff_option(
                &render_model,
                selection.get(),
                &draft,
                pricing.get().as_ref(),
            ),
        );
    });
    view! {
        <EChartsHost
            id=HOST_ID
            label=format!("{} deterministic mock payoff chart", model.symbol)
            class="min-h-[20rem] w-full flex-1 bg-canvas xl:min-h-0"
        />
    }
}

pub fn build_payoff_option(
    model: &AssetSimulationReadModel,
    selection: ScenarioSelection,
    draft: &[DraftLeg],
    pricing: Option<&StrategySimulationResult>,
) -> String {
    let selected_curve = model.time_payoffs.iter().min_by(|left, right| {
        (f64::from(left.elapsed_days) - selection.time_days)
            .abs()
            .total_cmp(&(f64::from(right.elapsed_days) - selection.time_days).abs())
    });
    let at_expiration =
        selected_curve.is_some_and(|curve| curve.elapsed_days == expiration_day(model));
    let prices = payoff_prices(model, draft);
    let draft_horizon = first_expiration(draft);
    let expiration_series = draft_horizon
        .map(|date| format!("{EXPIRATION_SERIES} ({})", short_expiration(date)))
        .unwrap_or_else(|| EXPIRATION_SERIES.to_owned());
    let progress = if expiration_day(model) == 0 {
        1.0
    } else {
        (selection.time_days / f64::from(expiration_day(model))).clamp(0.0, 1.0)
    };
    let fallback_current = prices
        .iter()
        .map(|price| {
            let current_proxy = position_current_proxy(draft, *price, model.current_spot);
            let horizon_value =
                position_horizon_pnl(draft, *price, model.current_spot, draft_horizon);
            vec![
                *price,
                current_proxy * (1.0 - progress) + horizon_value * progress,
            ]
        })
        .collect::<Vec<_>>();
    let fallback_expiration = prices
        .iter()
        .map(|price| {
            vec![
                *price,
                position_horizon_pnl(draft, *price, model.current_spot, draft_horizon),
            ]
        })
        .collect::<Vec<_>>();
    let backend_current = pricing.and_then(current_backend_curve);
    let backend_expiration = pricing.and_then(expiration_backend_curve);
    let current = backend_current
        .map(curve_points)
        .unwrap_or(fallback_current);
    let expiration = backend_expiration
        .map(curve_points)
        .unwrap_or(fallback_expiration);
    let breakevens = pricing
        .map(|result| result.break_even_points.clone())
        .unwrap_or_else(|| zero_crossings(&expiration));
    let backend_spot = pricing
        .map(|result| result.spot)
        .unwrap_or(model.current_spot);
    let payoff_legend = if draft.is_empty() || pricing.is_none() {
        Vec::new()
    } else {
        vec![
            json!(SELECTED_TIME_SERIES),
            json!(expiration_series.clone()),
        ]
    };
    let mut marker_legend = Vec::new();
    let mut series = Vec::new();
    if !draft.is_empty() && pricing.is_some() {
        marker_legend.extend([
            json!({ "name": "Spot", "icon": "rect" }),
            json!({ "name": "Scenario price", "icon": "rect" }),
        ]);
        series.extend([
            payoff_series(
                &expiration_series,
                expiration,
                tokens::TEXT_PRIMARY,
                true,
                0.0,
            ),
            payoff_area_series(
                "Profit area",
                &current,
                true,
                if at_expiration { 0.0 } else { 0.22 },
            ),
            payoff_area_series(
                "Loss area",
                &current,
                false,
                if at_expiration { 0.0 } else { 0.22 },
            ),
            payoff_series(
                SELECTED_TIME_SERIES,
                current,
                tokens::INTERACTIVE_TEXT,
                false,
                if at_expiration { 0.0 } else { 0.22 },
            ),
            marker_series("Spot", backend_spot, tokens::INTERACTIVE_TEXT, 0),
            marker_series(
                "Scenario price",
                selection.spot,
                tokens::FINANCE_POSITIVE,
                36,
            ),
        ]);
        if !breakevens.is_empty() {
            marker_legend.push(json!({ "name": "Breakeven", "icon": "rect" }));
            series.push(marker_series_many(
                "Breakeven",
                &breakevens,
                tokens::LEVEL_SPECIAL,
                18,
            ));
        }
    }
    let legend_items = payoff_legend
        .into_iter()
        .chain(marker_legend)
        .collect::<Vec<_>>();
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
        "legend": {
            "top": 8,
            "left": 20,
            "right": 20,
            "itemWidth": 22,
            "itemHeight": 2,
            "itemGap": 18,
            "selectedMode": "multiple",
            "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
            "data": legend_items
        },
        "tooltip": {
            "trigger": "axis",
            "backgroundColor": tokens::TEXT_SECONDARY,
            "borderColor": tokens::TEXT_MUTED_READABLE,
            "textStyle": { "color": tokens::CANVAS },
            "axisPointer": { "type": "cross" }
        },
        "grid": { "left": 62, "right": 28, "top": 76, "bottom": 50 },
        "xAxis": {
            "type": "value",
            "name": "Underlying Price (USD)",
            "nameLocation": "middle",
            "nameGap": 32,
            "min": "dataMin",
            "max": "dataMax",
            "axisLine": { "lineStyle": { "color": tokens::BORDER } },
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE },
            "splitLine": { "lineStyle": { "color": tokens::CHART_GRID, "type": "dashed" } }
        },
        "yAxis": {
            "type": "value",
            "name": "P&L (USD)",
            "nameLocation": "middle",
            "nameGap": 48,
            "nameRotate": 90,
            "nameTextStyle": { "color": tokens::TEXT_SECONDARY },
            "axisLine": { "show": false },
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE },
            "splitLine": { "lineStyle": { "color": tokens::CHART_GRID, "type": "dashed" } }
        },
        "series": series
    })
    .to_string()
}

fn current_backend_curve(result: &StrategySimulationResult) -> Option<&StrategySimulationCurve> {
    result
        .curves
        .iter()
        .find(|curve| curve.valuation_date == result.valuation_date)
        .or_else(|| {
            result
                .curves
                .iter()
                .min_by(|left, right| left.valuation_date.cmp(&right.valuation_date))
        })
}

fn expiration_backend_curve(result: &StrategySimulationResult) -> Option<&StrategySimulationCurve> {
    result
        .curves
        .iter()
        .find(|curve| curve.valuation_date == result.expiration)
        .or_else(|| {
            result
                .curves
                .iter()
                .max_by(|left, right| left.valuation_date.cmp(&right.valuation_date))
        })
}

fn curve_points(curve: &StrategySimulationCurve) -> Vec<Vec<f64>> {
    curve
        .points
        .iter()
        .map(|point| vec![point.spot, point.pnl])
        .collect()
}

pub fn backend_payoff_metrics(
    model: &AssetSimulationReadModel,
    draft: &[DraftLeg],
    pricing: Option<&StrategySimulationResult>,
) -> Vec<SimulationMetric> {
    let Some(result) = pricing else {
        let _ = model;
        return ["Max Profit", "Max Loss", "Breakeven", "POP", "Net Debit"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| SimulationMetric {
                label: label.to_owned(),
                value: "—".to_owned(),
                sentiment: match index {
                    0 => MetricSentiment::Positive,
                    1 => MetricSentiment::Negative,
                    2 => MetricSentiment::Special,
                    _ => MetricSentiment::Neutral,
                },
            })
            .collect();
    };
    let Some(curve) = expiration_backend_curve(result) else {
        return live_payoff_metrics(model, draft);
    };
    let minimum = curve
        .points
        .iter()
        .map(|point| point.pnl)
        .fold(f64::INFINITY, f64::min);
    let maximum = curve
        .points
        .iter()
        .map(|point| point.pnl)
        .fold(f64::NEG_INFINITY, f64::max);
    let opening_debit = opening_debit(draft);
    vec![
        SimulationMetric {
            label: "Max Profit".to_owned(),
            value: format_profit(maximum),
            sentiment: MetricSentiment::Positive,
        },
        SimulationMetric {
            label: "Max Loss".to_owned(),
            value: format_loss(minimum),
            sentiment: MetricSentiment::Negative,
        },
        SimulationMetric {
            label: "Breakeven".to_owned(),
            value: if result.break_even_points.is_empty() {
                "—".to_owned()
            } else {
                result
                    .break_even_points
                    .iter()
                    .map(|value| format!("{value:.2}"))
                    .collect::<Vec<_>>()
                    .join(" / ")
            },
            sentiment: MetricSentiment::Special,
        },
        SimulationMetric {
            label: "POP".to_owned(),
            value: "—".to_owned(),
            sentiment: MetricSentiment::Neutral,
        },
        SimulationMetric {
            label: if opening_debit >= 0.0 {
                "Net Debit".to_owned()
            } else {
                "Net Credit".to_owned()
            },
            value: format_money(opening_debit.abs()),
            sentiment: MetricSentiment::Neutral,
        },
    ]
}

pub fn backend_pnl_at_spot(result: &StrategySimulationResult, spot: f64) -> Option<f64> {
    let curve = current_backend_curve(result)?;
    curve
        .points
        .iter()
        .find(|point| (point.spot - spot).abs() < 0.000_001)
        .map(|point| point.pnl)
}

pub fn backend_greeks_at_spot(
    result: &StrategySimulationResult,
    spot: f64,
) -> Option<StrategySimulationGreeks> {
    let curve = current_backend_curve(result)?;
    curve
        .points
        .iter()
        .find(|point| (point.spot - spot).abs() < 0.000_001)
        .map(|point| point.greeks)
}

pub fn live_payoff_metrics(
    model: &AssetSimulationReadModel,
    draft: &[DraftLeg],
) -> Vec<SimulationMetric> {
    if draft.is_empty() {
        return ["Max Profit", "Max Loss", "Breakeven", "POP", "Net Debit"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| SimulationMetric {
                label: label.to_owned(),
                value: "—".to_owned(),
                sentiment: match index {
                    0 => MetricSentiment::Positive,
                    1 => MetricSentiment::Negative,
                    2 => MetricSentiment::Special,
                    _ => MetricSentiment::Neutral,
                },
            })
            .collect();
    }
    let horizon = first_expiration(draft);
    let upper = metric_upper_bound(model, draft);
    let points = (0..=1600)
        .map(|step| {
            let underlying = upper * f64::from(step) / 1600.0;
            vec![
                underlying,
                position_horizon_pnl(draft, underlying, model.current_spot, horizon),
            ]
        })
        .collect::<Vec<_>>();
    let minimum = points
        .iter()
        .map(|point| point[1])
        .fold(f64::INFINITY, f64::min);
    let maximum = points
        .iter()
        .map(|point| point[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let right_slope = right_tail_slope(draft);
    let crossings = zero_crossings(&points);
    let opening_debit = opening_debit(draft);
    let pop = model
        .metrics
        .iter()
        .find(|metric| metric.label == "POP")
        .map(|metric| format!("{} mock", metric.value))
        .unwrap_or_else(|| "Mock".to_owned());

    vec![
        SimulationMetric {
            label: "Max Profit".to_owned(),
            value: if right_slope > 0.000_001 {
                "∞".to_owned()
            } else {
                format_profit(maximum)
            },
            sentiment: MetricSentiment::Positive,
        },
        SimulationMetric {
            label: "Max Loss".to_owned(),
            value: if right_slope < -0.000_001 {
                "∞".to_owned()
            } else {
                format_loss(minimum)
            },
            sentiment: MetricSentiment::Negative,
        },
        SimulationMetric {
            label: "Breakeven".to_owned(),
            value: if crossings.is_empty() {
                "—".to_owned()
            } else {
                crossings
                    .iter()
                    .map(|value| format!("{value:.2}"))
                    .collect::<Vec<_>>()
                    .join(" / ")
            },
            sentiment: MetricSentiment::Special,
        },
        SimulationMetric {
            label: "POP".to_owned(),
            value: pop,
            sentiment: MetricSentiment::Neutral,
        },
        SimulationMetric {
            label: if opening_debit >= 0.0 {
                "Net Debit".to_owned()
            } else {
                "Net Credit".to_owned()
            },
            value: format_money(opening_debit.abs()),
            sentiment: MetricSentiment::Neutral,
        },
    ]
}

fn payoff_series(name: &str, data: Vec<Vec<f64>>, color: &str, dashed: bool, smooth: f64) -> Value {
    json!({
        "name": name,
        "type": "line",
        "showSymbol": false,
        "smooth": smooth,
        "data": data,
        "lineStyle": {
            "width": 2,
            "color": color,
            "type": if dashed { "dashed" } else { "solid" }
        }
    })
}

fn payoff_area_series(name: &str, data: &[Vec<f64>], positive: bool, smooth: f64) -> Value {
    let clipped = data
        .iter()
        .filter_map(|point| {
            let (spot, pnl) = (*point.first()?, *point.get(1)?);
            Some(vec![
                spot,
                if positive { pnl.max(0.0) } else { pnl.min(0.0) },
            ])
        })
        .collect::<Vec<_>>();
    let color = if positive {
        tokens::FINANCE_POSITIVE
    } else {
        tokens::FINANCE_NEGATIVE
    };
    json!({
        "name": name,
        "type": "line",
        "data": clipped,
        "showSymbol": false,
        "smooth": smooth,
        "silent": true,
        "tooltip": { "show": false },
        "lineStyle": { "width": 0, "opacity": 0 },
        "areaStyle": { "color": color, "opacity": 0.20 },
        "emphasis": { "disabled": true },
        "z": 1
    })
}

fn marker_series(name: &str, value: f64, color: &str, vertical_offset: i32) -> Value {
    json!({
        "name": name,
        "type": "line",
        "data": [],
        "symbol": "none",
        "showSymbol": false,
        "legendHoverLink": false,
        "lineStyle": { "width": 1, "color": color },
        "itemStyle": { "color": color },
        "markLine": {
            "symbol": ["none", "none"],
            "silent": true,
            "data": [marker(value, color, vertical_offset)]
        }
    })
}

fn marker_series_many(name: &str, values: &[f64], color: &str, vertical_offset: i32) -> Value {
    json!({
        "name": name,
        "type": "line",
        "data": [],
        "symbol": "none",
        "showSymbol": false,
        "legendHoverLink": false,
        "lineStyle": { "width": 1, "color": color },
        "itemStyle": { "color": color },
        "markLine": {
            "symbol": ["none", "none"],
            "silent": true,
            "data": values.iter().map(|value| marker(*value, color, vertical_offset)).collect::<Vec<_>>()
        }
    })
}

fn payoff_prices(model: &AssetSimulationReadModel, draft: &[DraftLeg]) -> Vec<f64> {
    let Some(first) = model.payoff.first() else {
        return Vec::new();
    };
    let Some(last) = model.payoff.last() else {
        return Vec::new();
    };
    let steps = ((last.underlying_price - first.underlying_price) / 0.5).ceil() as usize;
    let mut prices = (0..=steps)
        .map(|step| (first.underlying_price + step as f64 * 0.5).min(last.underlying_price))
        .collect::<Vec<_>>();
    prices.extend(draft.iter().filter_map(option_strike));
    prices.sort_by(f64::total_cmp);
    prices.dedup_by(|left, right| (*left - *right).abs() < 0.000_001);
    prices
}

fn position_horizon_pnl(
    legs: &[DraftLeg],
    underlying: f64,
    fallback_spot: f64,
    horizon: Option<&str>,
) -> f64 {
    legs.iter()
        .map(|leg| {
            let quantity = f64::from(leg.quantity);
            let entry = leg_price(leg).unwrap_or(fallback_spot);
            if leg.instrument.eq_ignore_ascii_case("CALL") {
                let strike = option_strike(leg).unwrap_or(underlying);
                if horizon.is_some_and(|date| date == leg.expiration.as_str()) {
                    quantity * ((underlying - strike).max(0.0) - entry) * 100.0
                } else {
                    quantity * (mock_remaining_value(underlying, strike, true) - entry) * 100.0
                }
            } else if leg.instrument.eq_ignore_ascii_case("PUT") {
                let strike = option_strike(leg).unwrap_or(underlying);
                if horizon.is_some_and(|date| date == leg.expiration.as_str()) {
                    quantity * ((strike - underlying).max(0.0) - entry) * 100.0
                } else {
                    quantity * (mock_remaining_value(underlying, strike, false) - entry) * 100.0
                }
            } else if leg.instrument.eq_ignore_ascii_case("STOCK") {
                quantity * (underlying - entry)
            } else {
                0.0
            }
        })
        .sum()
}

fn remaining_option_change(underlying: f64, current_spot: f64, strike: f64, call: bool) -> f64 {
    mock_remaining_value(underlying, strike, call)
        - mock_remaining_value(current_spot, strike, call)
}

fn mock_remaining_value(underlying: f64, strike: f64, call: bool) -> f64 {
    let signed_moneyness = if call {
        underlying - strike
    } else {
        strike - underlying
    };
    let smoothing = 8.0;
    0.5 * (signed_moneyness + (signed_moneyness * signed_moneyness + smoothing * smoothing).sqrt())
}

fn first_expiration(legs: &[DraftLeg]) -> Option<&str> {
    legs.iter()
        .filter(|leg| {
            leg.instrument.eq_ignore_ascii_case("CALL")
                || leg.instrument.eq_ignore_ascii_case("PUT")
        })
        .filter_map(|leg| expiration_key(&leg.expiration).map(|key| (key, leg.expiration.as_str())))
        .min_by_key(|(key, _)| *key)
        .map(|(_, expiration)| expiration)
}

fn expiration_key(value: &str) -> Option<(i32, u8, u8)> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let year = parts[2].parse().ok()?;
    let (day, month) = if let Ok(day) = parts[0].parse() {
        (day, month_number(parts[1])?)
    } else {
        (parts[1].parse().ok()?, month_number(parts[0])?)
    };
    Some((year, month, day))
}

fn month_number(value: &str) -> Option<u8> {
    match value.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn short_expiration(value: &str) -> String {
    value
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn position_current_proxy(legs: &[DraftLeg], underlying: f64, current_spot: f64) -> f64 {
    legs.iter()
        .map(|leg| {
            let quantity = f64::from(leg.quantity);
            if leg.instrument.eq_ignore_ascii_case("CALL") {
                let strike = option_strike(leg).unwrap_or(underlying);
                quantity * remaining_option_change(underlying, current_spot, strike, true) * 100.0
            } else if leg.instrument.eq_ignore_ascii_case("PUT") {
                let strike = option_strike(leg).unwrap_or(underlying);
                quantity * remaining_option_change(underlying, current_spot, strike, false) * 100.0
            } else if leg.instrument.eq_ignore_ascii_case("STOCK") {
                quantity * (underlying - current_spot)
            } else {
                0.0
            }
        })
        .sum()
}

fn option_strike(leg: &DraftLeg) -> Option<f64> {
    if leg.instrument.eq_ignore_ascii_case("CALL") || leg.instrument.eq_ignore_ascii_case("PUT") {
        numeric(&leg.strike)
    } else {
        None
    }
}

fn leg_price(leg: &DraftLeg) -> Option<f64> {
    numeric(&leg.price)
}

fn numeric(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_start_matches('$')
        .replace(',', "")
        .parse()
        .ok()
}

fn metric_upper_bound(model: &AssetSimulationReadModel, legs: &[DraftLeg]) -> f64 {
    let largest_strike = legs
        .iter()
        .filter_map(option_strike)
        .fold(0.0_f64, f64::max);
    model
        .payoff
        .last()
        .map(|point| point.underlying_price)
        .unwrap_or(model.current_spot)
        .max(model.current_spot)
        .max(largest_strike)
        * 4.0
}

fn right_tail_slope(legs: &[DraftLeg]) -> f64 {
    legs.iter()
        .map(|leg| {
            if leg.instrument.eq_ignore_ascii_case("CALL") {
                f64::from(leg.quantity) * 100.0
            } else if leg.instrument.eq_ignore_ascii_case("STOCK") {
                f64::from(leg.quantity)
            } else {
                0.0
            }
        })
        .sum()
}

fn opening_debit(legs: &[DraftLeg]) -> f64 {
    legs.iter()
        .map(|leg| {
            let multiplier = if leg.instrument.eq_ignore_ascii_case("STOCK") {
                1.0
            } else {
                100.0
            };
            f64::from(leg.quantity) * leg_price(leg).unwrap_or_default() * multiplier
        })
        .sum()
}

fn format_profit(value: f64) -> String {
    if !value.is_finite() || value <= 0.005 {
        "0".to_owned()
    } else {
        format!("+{}", format_money(value))
    }
}

fn format_loss(value: f64) -> String {
    if !value.is_finite() || value >= -0.005 {
        "0".to_owned()
    } else {
        let formatted = format_money(value.abs());
        format!("-{}", formatted.trim_start_matches('$'))
    }
}

fn format_money(value: f64) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn zero_crossings(points: &[Vec<f64>]) -> Vec<f64> {
    if points.iter().all(|point| point[1].abs() < 0.000_001) {
        return Vec::new();
    }
    let mut crossings = Vec::new();
    for pair in points.windows(2) {
        let (x0, y0) = (pair[0][0], pair[0][1]);
        let (x1, y1) = (pair[1][0], pair[1][1]);
        if y0.abs() < 0.000_001 {
            crossings.push(x0);
        } else if y0.signum() != y1.signum() {
            crossings.push(x0 + (0.0 - y0) * (x1 - x0) / (y1 - y0));
        }
    }
    if let Some(last) = points.last() {
        if last[1].abs() < 0.000_001 {
            crossings.push(last[0]);
        }
    }
    crossings.dedup_by(|left, right| (*left - *right).abs() < 0.01);
    crossings
}

fn expiration_day(model: &AssetSimulationReadModel) -> u8 {
    model
        .time_payoffs
        .iter()
        .map(|curve| curve.elapsed_days)
        .max()
        .unwrap_or_default()
}

fn marker(value: f64, color: &str, vertical_offset: i32) -> Value {
    json!({
        "xAxis": value,
        "lineStyle": { "color": color, "type": "dashed", "width": 1 },
        "label": {
            "show": true,
            "position": "insideEndTop",
            "offset": [0, vertical_offset],
            "rotate": 0,
            "formatter": format!("{value:.2}"),
            "color": color,
            "backgroundColor": tokens::CANVAS,
            "padding": [2, 4]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payoff_markers_preserve_provider_neutral_values() {
        let value = marker(192.8, tokens::LEVEL_SPECIAL, 18);
        assert_eq!(value["xAxis"], 192.8);
        assert_eq!(value["label"]["formatter"], "192.80");
        assert_eq!(value["label"]["offset"], json!([0, 18]));
        assert_eq!(
            marker_series("Breakeven", 192.8, tokens::LEVEL_SPECIAL, 18)["name"],
            "Breakeven"
        );
    }

    #[test]
    fn payoff_areas_only_clip_backend_values_at_zero() {
        let points = vec![vec![190.0, -25.0], vec![195.0, 40.0]];
        let profit = payoff_area_series("Profit", &points, true, 0.0);
        let loss = payoff_area_series("Loss", &points, false, 0.0);

        assert_eq!(profit["data"], json!([[190.0, 0.0], [195.0, 40.0]]));
        assert_eq!(loss["data"], json!([[190.0, -25.0], [195.0, 0.0]]));
        assert_eq!(profit["areaStyle"]["color"], tokens::FINANCE_POSITIVE);
        assert_eq!(loss["areaStyle"]["color"], tokens::FINANCE_NEGATIVE);
    }

    #[test]
    fn live_values_use_the_selected_backend_date_and_exact_spot() {
        let point = |pnl, delta| crate::ports::asset_simulation::StrategySimulationPoint {
            spot: 197.0,
            pnl,
            greeks: StrategySimulationGreeks {
                delta,
                ..Default::default()
            },
        };
        let result = StrategySimulationResult {
            valuation_date: "2025-05-14".to_owned(),
            expiration: "2025-05-17".to_owned(),
            spot: 191.13,
            break_even_points: Vec::new(),
            curves: vec![
                StrategySimulationCurve {
                    label: "selected".to_owned(),
                    valuation_date: "2025-05-14".to_owned(),
                    volatility_shift: 0.0,
                    points: vec![point(123.0, 0.25)],
                },
                StrategySimulationCurve {
                    label: "expiration".to_owned(),
                    valuation_date: "2025-05-17".to_owned(),
                    volatility_shift: 0.0,
                    points: vec![point(456.0, 0.75)],
                },
            ],
        };
        assert_eq!(backend_pnl_at_spot(&result, 197.0), Some(123.0));
        assert_eq!(backend_greeks_at_spot(&result, 197.0).unwrap().delta, 0.25);
    }

    #[test]
    fn expiration_payoff_uses_live_option_legs_and_finds_breakeven() {
        let legs = vec![DraftLeg {
            key: "call".into(),
            quantity: 1,
            instrument: "CALL".into(),
            strike: "190".into(),
            expiration: "17 May 2025".into(),
            price: "2.80".into(),
        }];
        let horizon = first_expiration(&legs);
        assert_eq!(position_horizon_pnl(&legs, 180.0, 191.13, horizon), -280.0);
        assert_eq!(position_horizon_pnl(&legs, 200.0, 191.13, horizon), 720.0);
        let points = vec![vec![190.0, -280.0], vec![200.0, 720.0]];
        let crossings = zero_crossings(&points);
        assert_eq!(crossings.len(), 1);
        assert!((crossings[0] - 192.8).abs() < 0.001);
    }

    #[test]
    fn stock_payoff_uses_quantity_and_entry_price() {
        let legs = vec![DraftLeg {
            key: "stock".into(),
            quantity: 100,
            instrument: "STOCK".into(),
            strike: "—".into(),
            expiration: "—".into(),
            price: "$191.13".into(),
        }];
        assert!((position_horizon_pnl(&legs, 198.0, 191.13, None) - 687.0).abs() < 0.001);
    }

    #[test]
    fn calendar_uses_the_first_expiration_as_analysis_horizon() {
        let legs = vec![
            DraftLeg {
                key: "near".into(),
                quantity: -1,
                instrument: "CALL".into(),
                strike: "190".into(),
                expiration: "17 May 2025".into(),
                price: "2.00".into(),
            },
            DraftLeg {
                key: "far".into(),
                quantity: 1,
                instrument: "CALL".into(),
                strike: "190".into(),
                expiration: "21 Jun 2025".into(),
                price: "4.00".into(),
            },
        ];
        assert_eq!(first_expiration(&legs), Some("17 May 2025"));
        assert!(position_horizon_pnl(&legs, 190.0, 191.13, first_expiration(&legs)).is_finite());
    }

    #[test]
    fn tail_and_opening_cash_flow_follow_live_draft_quantities() {
        let legs = vec![
            DraftLeg {
                key: "stock".into(),
                quantity: 100,
                instrument: "STOCK".into(),
                strike: "—".into(),
                expiration: "—".into(),
                price: "191.13".into(),
            },
            DraftLeg {
                key: "call".into(),
                quantity: -1,
                instrument: "CALL".into(),
                strike: "200".into(),
                expiration: "17 May 2025".into(),
                price: "0.80".into(),
            },
        ];
        assert_eq!(right_tail_slope(&legs), 0.0);
        assert!((opening_debit(&legs) - 19_033.0).abs() < 0.001);
        assert!(zero_crossings(&[vec![0.0, 0.0], vec![1.0, 0.0]]).is_empty());
    }
}
