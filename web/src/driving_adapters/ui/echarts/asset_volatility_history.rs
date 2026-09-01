use crate::{
    application::asset_volatility::{VolatilityHistoryPoint, VolatilityHistorySummary},
    design_system::tokens,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-volatility-history-echarts";

#[component]
pub fn VolatilityHistoryChart(
    history: Vec<VolatilityHistoryPoint>,
    summary: VolatilityHistorySummary,
) -> impl IntoView {
    let option = build_history_option(&history);
    Effect::new(move |_| {
        render_chart(HOST_ID, &option);
    });
    view! {
        <section class="flex h-full min-h-64 flex-col border border-border bg-surface" aria-label="Historical implied and realized volatility">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Historical IV vs Realized Volatility"</h2><span class="text-[0.6875rem] text-text-secondary">"Legend controls visibility"</span></div>
            <div class="relative min-h-56 flex-1 bg-canvas">
                <div class="pointer-events-none absolute right-16 top-2 z-10 flex gap-2" aria-label="Current volatility summary">
                    <div class="min-w-40 border border-border bg-surface-elevated/95 px-3 py-2 shadow-sm">
                        <p class="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.12em] text-text-secondary">"Current"</p>
                        <dl class="grid grid-cols-[1fr_auto] gap-x-4 gap-y-0.5 text-[0.6875rem] numeric">
                            <dt class="text-text-secondary">"ATM IV 30D"</dt><dd class="font-semibold text-volatility-high">{summary.atm_iv_30d}</dd>
                            <dt class="text-text-secondary">"RV20"</dt><dd class="font-semibold text-volatility-realized">{summary.rv20}</dd>
                            <dt class="text-text-secondary">"RV60"</dt><dd class="font-semibold text-interactive-text">{summary.rv60}</dd>
                        </dl>
                    </div>
                    <div class="min-w-36 border border-border bg-surface-elevated/95 px-3 py-2 shadow-sm">
                        <p class="text-[0.625rem] font-semibold uppercase tracking-[0.12em] text-text-secondary">"IV–RV Spread (30D)"</p>
                        <p class="mt-1 text-sm font-bold text-volatility-high numeric">{summary.iv_rv_spread_30d}</p>
                        <p class="mt-0.5 text-[0.6875rem] text-text-secondary">"Percentile "<span class="font-semibold text-text-primary numeric">{summary.percentile}</span></p>
                    </div>
                </div>
                <EChartsHost id=HOST_ID label="Historical ATM implied volatility and realized volatility" class="h-full min-h-56 w-full bg-canvas" />
            </div>
        </section>
    }
}

pub fn build_history_option(history: &[VolatilityHistoryPoint]) -> String {
    let values = history
        .iter()
        .flat_map(|point| {
            [
                point.atm_iv_30d_percent,
                point.realized_volatility_20d_percent,
                point.realized_volatility_60d_percent,
            ]
        })
        .collect::<Vec<_>>();
    let minimum = values.iter().copied().fold(f64::INFINITY, f64::min).floor() - 2.0;
    let maximum = values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        + 5.0;
    let earnings = history
        .iter()
        .enumerate()
        .filter(|(_, point)| point.earnings)
        .map(|(index, _)| json!([index, maximum - 3.0]))
        .collect::<Vec<_>>();
    let earnings_lines = history
        .iter()
        .enumerate()
        .filter(|(_, point)| point.earnings)
        .map(|(index, _)| json!({ "xAxis": index }))
        .collect::<Vec<_>>();
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "color": [tokens::VOLATILITY_HIGH, tokens::VOLATILITY_REALIZED, tokens::INTERACTIVE_TEXT, tokens::TEXT_SECONDARY],
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 10 },
        "legend": {
            "top": 8,
            "left": 20,
            "itemGap": 42,
            "itemWidth": 28,
            "selectedMode": "multiple",
            "textStyle": { "color": tokens::TEXT_SECONDARY },
            "data": ["ATM IV 30D", "RV20", "RV60", "Earnings"]
        },
        "tooltip": {
            "trigger": "axis",
            "backgroundColor": tokens::SURFACE_ELEVATED,
            "borderColor": tokens::BORDER,
            "textStyle": { "color": tokens::TEXT_PRIMARY },
            "axisPointer": { "type": "line" }
        },
        "grid": { "left": 48, "right": 18, "top": 94, "bottom": 42, "containLabel": false },
        "xAxis": {
            "type": "category",
            "boundaryGap": false,
            "data": history.iter().map(|point| point.label.clone()).collect::<Vec<_>>(),
            "axisLine": { "lineStyle": { "color": tokens::BORDER } },
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE, "interval": 2 },
            "splitLine": { "show": true, "lineStyle": { "color": tokens::CHART_GRID } }
        },
        "yAxis": {
            "type": "value",
            "name": "Volatility (%)",
            "min": minimum,
            "max": maximum,
            "axisLabel": { "color": tokens::TEXT_MUTED_READABLE },
            "splitLine": { "lineStyle": { "color": tokens::CHART_GRID } }
        },
        "series": [
            history_line("ATM IV 30D", history.iter().map(|point| point.atm_iv_30d_percent).collect(), tokens::VOLATILITY_HIGH),
            history_line("RV20", history.iter().map(|point| point.realized_volatility_20d_percent).collect(), tokens::VOLATILITY_REALIZED),
            history_line("RV60", history.iter().map(|point| point.realized_volatility_60d_percent).collect(), tokens::INTERACTIVE_TEXT),
            json!({
                "name": "Earnings",
                "type": "scatter",
                "data": earnings,
                "symbol": "circle",
                "symbolSize": 18,
                "label": { "show": true, "formatter": "E", "color": tokens::TEXT_PRIMARY, "fontSize": 9 },
                "itemStyle": { "color": tokens::CANVAS, "borderColor": tokens::TEXT_PRIMARY, "borderWidth": 1.2 },
                "markLine": { "silent": true, "symbol": ["none", "none"], "label": { "show": false }, "lineStyle": { "color": tokens::TEXT_MUTED_READABLE, "type": "dashed", "width": 1 }, "data": earnings_lines }
            })
        ]
    }).to_string()
}

fn history_line(name: &str, data: Vec<f64>, color: &str) -> Value {
    json!({ "name": name, "type": "line", "showSymbol": false, "data": data, "lineStyle": { "color": color, "width": 2 } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_uses_fixed_category_axis_and_grouped_earnings_series() {
        let points = vec![VolatilityHistoryPoint {
            label: "2024-01-01".into(),
            atm_iv_30d_percent: 25.0,
            realized_volatility_20d_percent: 18.0,
            realized_volatility_60d_percent: 17.0,
            earnings: true,
        }];
        let option = build_history_option(&points);
        assert!(option.contains("Earnings"));
        assert!(option.contains("markLine"));
        assert!(option.contains(tokens::VOLATILITY_REALIZED));
    }
}
