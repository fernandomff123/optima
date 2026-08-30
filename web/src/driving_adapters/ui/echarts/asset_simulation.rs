use crate::{
    application::asset_simulation::AssetSimulationReadModel, design_system::tokens,
    driving_adapters::ui::components::ScenarioSelection,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-simulation-payoff-echarts";
const SELECTED_TIME_SERIES: &str = "Selected time";
const EXPIRATION_SERIES: &str = "At expiration";

#[component]
pub fn SimulationPayoffChart(
    model: AssetSimulationReadModel,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let render_model = model.clone();
    Effect::new(move |_| {
        render_chart(
            HOST_ID,
            &build_payoff_option(&render_model, selection.get()),
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
) -> String {
    let selected_curve = model.time_payoffs.iter().min_by(|left, right| {
        (f64::from(left.elapsed_days) - selection.time_days)
            .abs()
            .total_cmp(&(f64::from(right.elapsed_days) - selection.time_days).abs())
    });
    let selected_values = selected_curve
        .map(|curve| curve.pnl_values.as_slice())
        .unwrap_or(&[]);
    let at_expiration =
        selected_curve.is_some_and(|curve| curve.elapsed_days == expiration_day(model));
    let current = model
        .payoff
        .iter()
        .enumerate()
        .map(|(index, point)| {
            vec![
                point.underlying_price,
                selected_values
                    .get(index)
                    .copied()
                    .unwrap_or(point.current_pnl),
            ]
        })
        .collect::<Vec<_>>();
    let expiration = model
        .payoff
        .iter()
        .map(|point| vec![point.underlying_price, point.expiration_pnl])
        .collect::<Vec<_>>();
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
        "legend": [
            {
                "top": 8,
                "left": 20,
                "itemGap": 36,
                "selectedMode": "multiple",
                "textStyle": { "color": tokens::TEXT_SECONDARY },
                "data": [SELECTED_TIME_SERIES, EXPIRATION_SERIES]
            },
            {
                "top": 8,
                "left": 310,
                "itemWidth": 22,
                "itemHeight": 2,
                "itemGap": 36,
                "selectedMode": false,
                "textStyle": { "color": tokens::TEXT_SECONDARY },
                "data": [
                    { "name": "Spot", "icon": "rect" },
                    { "name": "Scenario price", "icon": "rect" },
                    { "name": "Breakeven", "icon": "rect" }
                ]
            }
        ],
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
        "series": [
            payoff_series(
                EXPIRATION_SERIES,
                expiration,
                tokens::TEXT_PRIMARY,
                true,
                0.0,
            ),
            payoff_series(
                SELECTED_TIME_SERIES,
                current,
                tokens::INTERACTIVE_TEXT,
                false,
                if at_expiration { 0.0 } else { 0.22 },
            ),
            marker_series("Spot", model.current_spot, tokens::INTERACTIVE_TEXT, 0),
            marker_series("Scenario price", selection.spot, tokens::FINANCE_POSITIVE, 36),
            marker_series("Breakeven", model.breakeven, tokens::LEVEL_SPECIAL, 18)
        ]
    })
    .to_string()
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
        },
        "areaStyle": if dashed { Value::Null } else { json!({ "color": tokens::STATE_SELECTED, "opacity": 0.34 }) }
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
}
