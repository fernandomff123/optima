use crate::{
    application::asset_volatility::{VolatilitySmile, VolatilityTermPoint},
    design_system::tokens,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const SMILE_HOST_ID: &str = "asset-volatility-smiles-echarts";
const TERM_HOST_ID: &str = "asset-volatility-term-echarts";

#[component]
pub fn VolatilityAnalytics(
    moneyness: Vec<f64>,
    smiles: Vec<VolatilitySmile>,
    term_structure: Vec<VolatilityTermPoint>,
) -> impl IntoView {
    let smile_option = build_smile_option(&moneyness, &smiles);
    let term_option = build_term_option(&term_structure);
    Effect::new(move |_| {
        render_chart(SMILE_HOST_ID, &smile_option);
        render_chart(TERM_HOST_ID, &term_option);
    });
    view! {
        <section class="grid h-full min-h-0 border border-border bg-surface xl:grid-cols-2" aria-label="Coordinated volatility views">
            <div class="flex min-h-64 flex-col border-b border-border xl:min-h-0 xl:border-b-0 xl:border-r"><div class="panel-header"><h2 class="text-sm font-semibold">"IV Smile by Expiry (Calls + Puts)"</h2></div><EChartsHost id=SMILE_HOST_ID label="Implied volatility smiles for selected expiries" class="min-h-56 flex-1 bg-canvas" /></div>
            <div class="flex min-h-64 flex-col xl:min-h-0"><div class="panel-header"><h2 class="text-sm font-semibold">"ATM IV Term Structure"</h2></div><EChartsHost id=TERM_HOST_ID label="ATM implied volatility term structure" class="min-h-56 flex-1 bg-canvas" /></div>
        </section>
    }
}

pub fn build_smile_option(moneyness: &[f64], smiles: &[VolatilitySmile]) -> String {
    let colors = [
        tokens::INTERACTIVE_TEXT,
        tokens::VOLATILITY_HIGH,
        tokens::VOLATILITY_REALIZED,
        tokens::LEVEL_SPECIAL,
    ];
    let series = smiles.iter().enumerate().map(|(index, curve)| {
        json!({
            "name": curve.label,
            "type": "line",
            "showSymbol": true,
            "symbolSize": 5,
            "data": moneyness.iter().zip(&curve.implied_volatility_percent).map(|(money, value)| vec![*money, *value]).collect::<Vec<_>>(),
            "lineStyle": { "color": colors[index % colors.len()], "width": 2 },
            "itemStyle": { "color": colors[index % colors.len()] }
        })
    }).collect::<Vec<Value>>();
    line_option("Moneyness", "Implied Volatility (%)", series, true)
}

pub fn build_term_option(term: &[VolatilityTermPoint]) -> String {
    let data = term
        .iter()
        .map(|point| {
            vec![
                f64::from(point.days_to_expiry),
                point.implied_volatility_percent,
            ]
        })
        .collect::<Vec<_>>();
    line_option(
        "Days to Expiry",
        "ATM IV (%)",
        vec![
            json!({ "name": "ATM IV", "type": "line", "showSymbol": true, "symbolSize": 6, "data": data,
            "label": { "show": true, "position": "top", "formatter": "{@[1]}%", "color": tokens::TEXT_SECONDARY, "fontSize": 9 },
            "labelLayout": { "hideOverlap": true }, "lineStyle": { "color": tokens::INTERACTIVE_TEXT, "width": 2 }, "itemStyle": { "color": tokens::INTERACTIVE_TEXT } }),
        ],
        false,
    )
}

fn line_option(x_name: &str, y_name: &str, series: Vec<Value>, show_legend: bool) -> String {
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 10 },
        "legend": { "show": show_legend, "top": 4, "left": "center", "textStyle": { "color": tokens::TEXT_SECONDARY } },
        "tooltip": { "trigger": "axis", "backgroundColor": tokens::SURFACE_ELEVATED, "borderColor": tokens::BORDER, "textStyle": { "color": tokens::TEXT_PRIMARY } },
        "grid": { "left": 52, "right": 18, "top": if show_legend { 38 } else { 18 }, "bottom": 46 },
        "xAxis": { "type": "value", "name": x_name, "nameLocation": "middle", "nameGap": 30, "axisLabel": { "color": tokens::TEXT_MUTED_READABLE }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "yAxis": { "type": "value", "name": y_name, "axisLabel": { "color": tokens::TEXT_MUTED_READABLE }, "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } } },
        "series": series
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytics_options_preserve_provider_neutral_values() {
        let smile = build_smile_option(
            &[1.0],
            &[VolatilitySmile {
                label: "30D".into(),
                days_to_expiry: 30,
                implied_volatility_percent: vec![24.4],
            }],
        );
        let term = build_term_option(&[VolatilityTermPoint {
            days_to_expiry: 30,
            implied_volatility_percent: 24.4,
        }]);
        assert!(smile.contains("24.4"));
        assert!(term.contains("30.0"));
        assert!(term.contains("hideOverlap"));
    }
}
