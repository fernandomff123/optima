use crate::{application::asset_volatility::VolatilityGrid, design_system::tokens};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-volatility-heatmap-echarts";

#[component]
pub fn VolatilityHeatmapChart(grid: VolatilityGrid) -> impl IntoView {
    let option = build_heatmap_option(&grid);
    Effect::new(move |_| {
        render_chart(HOST_ID, &option);
    });
    view! {
        <EChartsHost
            id=HOST_ID
            label="Implied volatility heatmap by moneyness and days to expiry"
            class="min-h-[25rem] w-full flex-1 bg-canvas"
        />
    }
}

pub fn build_heatmap_option(grid: &VolatilityGrid) -> String {
    let values = grid
        .implied_volatility_percent
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let data = grid
        .implied_volatility_percent
        .iter()
        .enumerate()
        .flat_map(|(row, values)| {
            values.iter().enumerate().map(move |(column, value)| {
                let selected =
                    row == grid.selected_moneyness_index && column == grid.selected_expiry_index;
                if selected {
                    json!({
                        "value": [column, row, value],
                        "itemStyle": { "borderColor": tokens::INTERACTIVE_TEXT, "borderWidth": 2 }
                    })
                } else {
                    json!([column, row, value])
                }
            })
        })
        .collect::<Vec<Value>>();
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
        "tooltip": {
            "position": "top",
            "backgroundColor": tokens::SURFACE_ELEVATED,
            "borderColor": tokens::BORDER,
            "textStyle": { "color": tokens::TEXT_PRIMARY }
        },
        "grid": { "left": 88, "right": 22, "top": 78, "bottom": 72 },
        "xAxis": {
            "type": "category",
            "position": "top",
            "name": "Days to Expiry",
            "nameLocation": "middle",
            "nameGap": 34,
            "nameTextStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
            "data": grid.days_to_expiry.iter().map(|day| format!("{day} DTE")).collect::<Vec<_>>(),
            "axisLine": { "show": false },
            "axisTick": { "show": false },
            "axisLabel": { "color": tokens::TEXT_SECONDARY }
        },
        "yAxis": {
            "type": "category",
            "name": "Moneyness",
            "nameLocation": "start",
            "nameGap": 22,
            "data": grid.moneyness.iter().map(|value| format!("{value:.2}")).collect::<Vec<_>>(),
            "axisLine": { "show": false },
            "axisTick": { "show": false },
            "axisLabel": { "color": tokens::TEXT_PRIMARY }
        },
        "visualMap": {
            "min": minimum,
            "max": maximum,
            "calculable": false,
            "orient": "horizontal",
            "left": "center",
            "bottom": 16,
            "itemWidth": 9,
            "itemHeight": 250,
            "text": [format!("{maximum:.1}%"), format!("{minimum:.1}%")],
            "textStyle": { "color": tokens::TEXT_SECONDARY },
            "inRange": { "color": [tokens::VOLATILITY_LOW, tokens::INTERACTIVE_SOURCE, tokens::VOLATILITY_HIGH] }
        },
        "series": [{
            "name": "Implied Volatility",
            "type": "heatmap",
            "data": data,
            "label": { "show": true, "color": tokens::TEXT_PRIMARY, "formatter": "{@[2]}%" },
            "itemStyle": { "borderColor": tokens::BORDER, "borderWidth": 1 },
            "emphasis": { "itemStyle": { "shadowBlur": 8, "shadowColor": tokens::CANVAS } }
        }]
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heatmap_keeps_axes_values_and_selected_cell() {
        let grid = VolatilityGrid {
            moneyness: vec![0.9, 1.0],
            days_to_expiry: vec![7, 30],
            implied_volatility_percent: vec![vec![25.0, 24.0], vec![23.0, 22.0]],
            selected_moneyness_index: 1,
            selected_expiry_index: 1,
        };
        let option = build_heatmap_option(&grid);
        assert!(option.contains("30 DTE"));
        assert!(option.contains("Days to Expiry"));
        assert!(option.contains("itemHeight\":250"));
        assert!(option.contains(tokens::VOLATILITY_HIGH));
        assert!(option.contains("borderWidth\":2"));
    }
}
