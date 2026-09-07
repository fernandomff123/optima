use crate::{application::asset_volatility::VolatilityGrid, design_system::tokens};
use leptos::prelude::*;
use serde_json::json;

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-volatility-surface-echarts";

#[component]
pub fn VolatilitySurfaceChart(grid: VolatilityGrid) -> impl IntoView {
    let option = build_surface_option(&grid);
    Effect::new(move |_| {
        render_chart(HOST_ID, &option);
    });
    view! {
        <EChartsHost
            id=HOST_ID
            label="Illustrative AAPL implied volatility surface by moneyness and days to expiry"
            class="min-h-[25rem] w-full flex-1 bg-canvas"
        />
    }
}

pub fn build_surface_option(grid: &VolatilityGrid) -> String {
    let data = grid
        .moneyness
        .iter()
        .enumerate()
        .flat_map(|(row, money)| {
            grid.days_to_expiry
                .iter()
                .enumerate()
                .map(move |(column, days)| {
                    vec![
                        *money,
                        f64::from(*days),
                        grid.implied_volatility_percent[row][column],
                    ]
                })
        })
        .collect::<Vec<_>>();
    let minimum = data
        .iter()
        .map(|point| point[2])
        .fold(f64::INFINITY, f64::min);
    let maximum = data
        .iter()
        .map(|point| point[2])
        .fold(f64::NEG_INFINITY, f64::max);
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "tooltip": { "show": true, "backgroundColor": tokens::SURFACE_ELEVATED, "borderColor": tokens::BORDER, "textStyle": { "color": tokens::TEXT_PRIMARY } },
        "legend": {
            "show": true,
            "top": 10,
            "right": 24,
            "itemWidth": 12,
            "itemHeight": 12,
            "data": ["Observed IV"],
            "textStyle": { "color": tokens::TEXT_SECONDARY }
        },
        "visualMap": {
            "show": false,
            "min": minimum,
            "max": maximum,
            "dimension": 2,
            "seriesIndex": 0,
            "orient": "vertical",
            "right": 18,
            "top": "middle",
            "itemHeight": 160,
            "textStyle": { "color": tokens::TEXT_SECONDARY },
            "inRange": { "color": [tokens::VOLATILITY_LOW, tokens::INTERACTIVE_SOURCE, tokens::VOLATILITY_HIGH] }
        },
        "xAxis3D": { "type": "value", "name": "Moneyness", "min": 0.5, "max": 1.5, "interval": 0.25, "nameGap": 24, "nameTextStyle": { "color": tokens::TEXT_PRIMARY }, "axisLabel": { "color": tokens::TEXT_MUTED_READABLE }, "axisLine": { "lineStyle": { "color": tokens::BORDER } }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "yAxis3D": { "type": "value", "name": "Days to Expiry", "min": 0, "max": 365, "nameGap": 28, "nameTextStyle": { "color": tokens::TEXT_PRIMARY }, "axisLabel": { "color": tokens::TEXT_MUTED_READABLE }, "axisLine": { "lineStyle": { "color": tokens::BORDER } }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "zAxis3D": { "type": "value", "name": "Implied Volatility (%)", "nameGap": 22, "nameTextStyle": { "color": tokens::TEXT_PRIMARY }, "axisLabel": { "color": tokens::TEXT_MUTED_READABLE, "formatter": "{value}%" }, "axisLine": { "lineStyle": { "color": tokens::BORDER } }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "grid3D": {
            "left": 20,
            "right": 20,
            "top": 8,
            "bottom": 8,
            "boxWidth": 145,
            "boxDepth": 115,
            "boxHeight": 60,
            "environment": tokens::CANVAS,
            "viewControl": { "projection": "perspective", "alpha": 18, "beta": 38, "distance": 145, "rotateSensitivity": 0, "zoomSensitivity": 0, "panSensitivity": 0 },
            "light": { "main": { "intensity": 1.15, "shadow": false }, "ambient": { "intensity": 0.45 } }
        },
        "series": [
            {
                "name": "IV Surface",
                "type": "surface",
                "data": data.clone(),
                "wireframe": { "show": true, "lineStyle": { "color": tokens::BORDER, "width": 1 } },
                "shading": "lambert"
            },
            {
                "name": "Observed IV",
                "type": "scatter3D",
                "data": data,
                "symbol": "circle",
                "symbolSize": 4,
                "itemStyle": { "color": tokens::TEXT_PRIMARY, "opacity": 1 }
            }
        ]
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_option_uses_fixed_echarts_gl_perspective() {
        let grid = VolatilityGrid {
            moneyness: vec![1.0],
            days_to_expiry: vec![30],
            implied_volatility_percent: vec![vec![24.4]],
            selected_moneyness_index: 0,
            selected_expiry_index: 0,
        };
        let option = build_surface_option(&grid);
        assert!(option.contains("surface"));
        assert!(option.contains("rotateSensitivity\":0"));
        assert!(option.contains("scatter3D"));
        assert!(option.contains("Observed IV"));
        assert!(option.contains("seriesIndex\":0"));
        assert!(option.contains("symbolSize\":4"));
        assert!(option.contains("beta\":38"));
        assert!(option.contains("24.4"));
    }
}
