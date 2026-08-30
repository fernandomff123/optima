use crate::{
    application::asset_simulation::AssetSimulationReadModel, design_system::tokens,
    driving_adapters::ui::components::ScenarioSelection,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-simulation-payoff-echarts";

#[component]
pub fn SimulationPayoffChart(
    model: AssetSimulationReadModel,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let render_model = model.clone();
    Effect::new(move |_| {
        render_chart(
            HOST_ID,
            &build_payoff_option(&render_model, selection.get().spot),
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

pub fn build_payoff_option(model: &AssetSimulationReadModel, scenario_spot: f64) -> String {
    let current = model
        .payoff
        .iter()
        .map(|point| vec![point.underlying_price, point.current_pnl])
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
        "legend": {
            "top": 8,
            "left": 20,
            "textStyle": { "color": tokens::TEXT_SECONDARY },
            "data": [
                format!("Current ({})", model.current_date),
                format!("At Expiration ({})", model.expiration_date)
            ]
        },
        "tooltip": {
            "trigger": "axis",
            "backgroundColor": tokens::TEXT_SECONDARY,
            "borderColor": tokens::TEXT_MUTED_READABLE,
            "textStyle": { "color": tokens::CANVAS },
            "axisPointer": { "type": "cross" }
        },
        "grid": { "left": 62, "right": 28, "top": 48, "bottom": 50 },
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
            "nameTextStyle": { "color": tokens::TEXT_SECONDARY },
            "axisLine": { "show": false },
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE },
            "splitLine": { "lineStyle": { "color": tokens::CHART_GRID, "type": "dashed" } }
        },
        "series": [
            payoff_series(
                &format!("Current ({})", model.current_date),
                current,
                tokens::INTERACTIVE_TEXT,
                false,
                model,
                scenario_spot,
            ),
            payoff_series(
                &format!("At Expiration ({})", model.expiration_date),
                expiration,
                tokens::TEXT_PRIMARY,
                true,
                model,
                scenario_spot,
            )
        ]
    })
    .to_string()
}

fn payoff_series(
    name: &str,
    data: Vec<Vec<f64>>,
    color: &str,
    dashed: bool,
    model: &AssetSimulationReadModel,
    scenario_spot: f64,
) -> Value {
    json!({
        "name": name,
        "type": "line",
        "showSymbol": false,
        "smooth": 0.22,
        "data": data,
        "lineStyle": {
            "width": 2,
            "color": color,
            "type": if dashed { "dashed" } else { "solid" }
        },
        "areaStyle": if dashed { Value::Null } else { json!({ "color": tokens::STATE_SELECTED, "opacity": 0.34 }) },
        "markLine": if dashed { Value::Null } else { json!({
            "symbol": ["none", "none"],
            "silent": true,
            "data": [
                marker("Spot", model.current_spot, tokens::INTERACTIVE_TEXT),
                marker("Scenario", scenario_spot, tokens::FINANCE_POSITIVE),
                marker("Breakeven", model.breakeven, tokens::LEVEL_SPECIAL)
            ]
        }) }
    })
}

fn marker(label: &str, value: f64, color: &str) -> Value {
    json!({
        "name": label,
        "xAxis": value,
        "lineStyle": { "color": color, "type": "dashed", "width": 1 },
        "label": { "show": true, "formatter": format!("{label} {value:.2}"), "color": color }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payoff_markers_preserve_provider_neutral_values() {
        let value = marker("Breakeven", 192.8, tokens::LEVEL_SPECIAL);
        assert_eq!(value["xAxis"], 192.8);
        assert_eq!(value["label"]["formatter"], "Breakeven 192.80");
    }
}
