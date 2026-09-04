use crate::{
    application::asset_simulation::AssetSimulationReadModel,
    design_system::tokens,
    driving_adapters::ui::components::ScenarioSelection,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const DATE_HOST_ID: &str = "asset-simulation-pnl-date-echarts";
const DECAY_HOST_ID: &str = "asset-simulation-time-decay-echarts";

#[component]
pub fn SimulationPnlByDateChart(
    model: AssetSimulationReadModel,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let render_model = model.clone();
    Effect::new(move |_| {
        render_chart(
            DATE_HOST_ID,
            &build_pnl_by_date_option(&render_model, selection.get()),
        );
    });
    view! {
        <EChartsHost id=DATE_HOST_ID label=format!("{} mock profit and loss by date", model.symbol) class="min-h-[18rem] h-full w-full bg-canvas" />
    }
}

#[component]
pub fn SimulationTimeDecayChart(
    model: AssetSimulationReadModel,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let render_model = model.clone();
    Effect::new(move |_| {
        render_chart(
            DECAY_HOST_ID,
            &build_time_decay_option(&render_model, selection.get()),
        );
    });
    view! {
        <EChartsHost id=DECAY_HOST_ID label=format!("{} deterministic mock time decay", model.symbol) class="min-h-[12rem] h-full w-full bg-canvas" />
    }
}

pub fn build_pnl_by_date_option(
    model: &AssetSimulationReadModel,
    selection: ScenarioSelection,
) -> String {
    let prices = model.payoff.iter().map(|point| point.underlying_price).collect::<Vec<_>>();
    let colors = [tokens::INTERACTIVE_TEXT, tokens::VOLATILITY_REALIZED, tokens::VOLATILITY_HIGH, tokens::LEVEL_SPECIAL];
    let mut series = model.time_payoffs.iter().enumerate().map(|(index, curve)| {
        line_series(
            &curve.label,
            prices.iter().copied().zip(curve.pnl_values.iter().copied()).map(|(price, pnl)| vec![price, pnl]).collect(),
            colors[index % colors.len()],
            false,
        )
    }).collect::<Vec<_>>();
    series.push(line_series(
        "At expiration",
        model.payoff.iter().map(|point| vec![point.underlying_price, point.expiration_pnl]).collect(),
        tokens::TEXT_PRIMARY,
        true,
    ));
    series.push(marker_series("Current", model.current_spot, tokens::INTERACTIVE_TEXT));
    series.push(marker_series("Breakeven", model.breakeven, tokens::LEVEL_SPECIAL));
    series.push(marker_series("Scenario", selection.spot, tokens::FINANCE_POSITIVE));
    chart_option("P&L by Underlying Price and Date", "Underlying Price (USD)", series).to_string()
}

pub fn build_time_decay_option(
    model: &AssetSimulationReadModel,
    selection: ScenarioSelection,
) -> String {
    let prices = model.payoff.iter().map(|point| point.underlying_price).collect::<Vec<_>>();
    let targets = [
        ("Spot -10%", model.current_spot * 0.90, tokens::NEGATIVE_TEXT),
        ("Spot", model.current_spot, tokens::INTERACTIVE_TEXT),
        ("Spot +5%", model.current_spot * 1.05, tokens::VOLATILITY_REALIZED),
        ("Spot +10%", model.current_spot * 1.10, tokens::VOLATILITY_HIGH),
    ];
    let mut series = targets.into_iter().map(|(label, target, color)| {
        let index = nearest(&prices, target);
        let data = model.time_payoffs.iter().map(|curve| vec![f64::from(curve.elapsed_days), curve.pnl_values.get(index).copied().unwrap_or_default()]).collect();
        line_series(label, data, color, false)
    }).collect::<Vec<_>>();
    series.push(marker_series("Selected time", selection.time_days, tokens::LEVEL_SPECIAL));
    chart_option("Time Decay at Selected Prices", "Days to Expiration", series).to_string()
}

fn chart_option(title: &str, x_name: &str, series: Vec<Value>) -> Value {
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "title": { "text": title, "left": 14, "top": 10, "textStyle": { "color": tokens::TEXT_PRIMARY, "fontSize": 12, "fontWeight": 600 } },
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 10 },
        "legend": { "top": 10, "right": 18, "itemGap": 20, "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 10 } },
        "tooltip": { "trigger": "axis", "backgroundColor": tokens::SURFACE_ELEVATED, "borderColor": tokens::BORDER, "textStyle": { "color": tokens::TEXT_PRIMARY } },
        "grid": { "left": 58, "right": 24, "top": 62, "bottom": 46 },
        "xAxis": { "type": "value", "name": x_name, "nameLocation": "middle", "nameGap": 30, "axisLabel": { "color": tokens::TEXT_MUTED_READABLE }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "yAxis": { "type": "value", "name": "P&L (USD)", "axisLabel": { "color": tokens::TEXT_MUTED_READABLE }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "series": series,
    })
}

fn line_series(name: &str, data: Vec<Vec<f64>>, color: &str, dashed: bool) -> Value {
    json!({ "name": name, "type": "line", "showSymbol": false, "smooth": 0.22, "data": data, "lineStyle": { "width": 2, "color": color, "type": if dashed { "dashed" } else { "solid" } } })
}

fn marker_series(name: &str, value: f64, color: &str) -> Value {
    json!({ "name": name, "type": "line", "data": [], "showSymbol": false, "lineStyle": { "color": color }, "markLine": { "symbol": ["none", "none"], "silent": true, "label": { "color": color, "formatter": format!("{value:.2}") }, "lineStyle": { "color": color, "type": "dashed", "width": 1 }, "data": [{ "xAxis": value }] } })
}

fn nearest(values: &[f64], target: f64) -> usize {
    values.iter().enumerate().min_by(|(_, left), (_, right)| (**left - target).abs().total_cmp(&(**right - target).abs())).map(|(index, _)| index).unwrap_or_default()
}
