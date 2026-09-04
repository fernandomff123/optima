use crate::{application::asset_options::VolatilitySmile, design_system::tokens};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-options-iv-smile-echarts";

#[component]
pub fn OptionsSmileChart(smile: VolatilitySmile) -> impl IntoView {
    let description = smile.description.clone();
    let option = build_options_smile_option(&smile);
    Effect::new(move |_| {
        render_chart(HOST_ID, &option);
    });
    view! {
        <div class="flex min-h-0 flex-1 flex-col">
            <div class="flex h-11 shrink-0 items-center gap-6 border-b border-border px-4 text-xs font-medium"><span class="flex h-11 items-center border-b-2 border-interactive-text text-text-primary">"IV Smile"</span>{["Open Interest", "Volume", "Put/Call"].into_iter().map(|label| view! { <button class="cursor-not-allowed text-text-secondary opacity-60" type="button" disabled>{label}</button> }).collect_view()}</div>
            <EChartsHost id=HOST_ID label=description.clone() class="min-h-64 w-full flex-1 bg-canvas xl:min-h-0" />
            <p class="sr-only">{description}</p>
        </div>
    }
}

pub fn build_options_smile_option(smile: &VolatilitySmile) -> String {
    let call_data = smile_points(&smile.strikes, &smile.call_iv);
    let put_data = smile_points(&smile.strikes, &smile.put_iv);
    let (strike_min, strike_max) = strike_bounds(&smile.strikes);
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "color": [tokens::INTERACTIVE_TEXT, tokens::FINANCE_NEGATIVE],
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
        "legend": {
            "top": 8,
            "left": 18,
            "itemGap": 30,
            "textStyle": { "color": tokens::TEXT_SECONDARY },
            "data": ["Call IV", "Put IV"]
        },
        "tooltip": {
            "trigger": "axis",
            "backgroundColor": tokens::SURFACE_ELEVATED,
            "borderColor": tokens::BORDER,
            "textStyle": { "color": tokens::TEXT_PRIMARY },
            "axisPointer": { "type": "cross" }
        },
        "grid": { "left": 70, "right": 24, "top": 52, "bottom": 48 },
        "xAxis": {
            "type": "value",
            "scale": true,
            "min": strike_min,
            "max": strike_max,
            "name": "Strike",
            "nameLocation": "middle",
            "nameGap": 32,
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE },
            "axisLine": { "lineStyle": { "color": tokens::BORDER } },
            "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } }
        },
        "yAxis": {
            "type": "value",
            "name": "Implied Volatility (%)",
            "nameLocation": "middle",
            "nameGap": 48,
            "nameRotate": 90,
            "min": 15,
            "max": 30,
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE, "formatter": "{value}%" },
            "axisLine": { "lineStyle": { "color": tokens::BORDER } },
            "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } }
        },
        "series": [
            {
                "name": "Call IV",
                "type": "line",
                "showSymbol": true,
                "symbolSize": 6,
                "data": call_data,
                "lineStyle": { "color": tokens::INTERACTIVE_TEXT, "width": 2 },
                "itemStyle": { "color": tokens::INTERACTIVE_TEXT },
                "markLine": {
                    "silent": true,
                    "symbol": ["none", "none"],
                    "lineStyle": { "color": tokens::LEVEL_SPECIAL, "type": "dashed", "width": 1 },
                    "label": { "show": true, "formatter": smile.spot_label.clone(), "color": tokens::LEVEL_SPECIAL, "position": "insideEndTop", "rotate": 0, "distance": 6 },
                    "data": [{ "xAxis": smile.spot }]
                }
            },
            {
                "name": "Put IV",
                "type": "line",
                "showSymbol": true,
                "symbolSize": 6,
                "data": put_data,
                "lineStyle": { "color": tokens::FINANCE_NEGATIVE, "width": 2 },
                "itemStyle": { "color": tokens::FINANCE_NEGATIVE }
            }
        ]
    }).to_string()
}

fn strike_bounds(strikes: &[f64]) -> (f64, f64) {
    let minimum = strikes.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let maximum = strikes.iter().copied().reduce(f64::max).unwrap_or(1.0);
    let padding = strikes
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .filter(|step| *step > 0.0)
        .reduce(f64::min)
        .unwrap_or_else(|| ((maximum - minimum).abs() * 0.05).max(1.0));
    (minimum - padding, maximum + padding)
}

fn smile_points(strikes: &[f64], values: &[f64]) -> Vec<Value> {
    strikes
        .iter()
        .zip(values)
        .map(|(strike, value)| json!([strike, value]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_preserves_provider_neutral_smile_values_and_spot_marker() {
        let smile = VolatilitySmile {
            strikes: vec![190.0, 192.5],
            call_iv: vec![20.1, 19.8],
            put_iv: vec![19.4, 19.1],
            spot: 191.13,
            spot_label: "191.13".into(),
            description: "Mock smile".into(),
        };
        let option = build_options_smile_option(&smile);
        assert!(option.contains("190.0"));
        assert!(option.contains("20.1"));
        assert!(option.contains("19.4"));
        assert!(option.contains("191.13"));
        assert!(option.contains("markLine"));
        assert!(option.contains("\"min\":187.5"));
        assert!(option.contains("\"max\":195.0"));
    }
}
