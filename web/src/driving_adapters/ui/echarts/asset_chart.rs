use crate::{
    application::asset_chart::{AssetChartReadModel, ChartIndicator, ChartTone},
    design_system::tokens,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::{EChartsHost, render_chart};

const HOST_ID: &str = "asset-chart-echarts";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChartVisibility {
    pub ma20: bool,
    pub ma50: bool,
    pub ma200: bool,
    pub rsi: bool,
    pub macd: bool,
}

#[component]
pub fn AssetChartCanvas(
    model: AssetChartReadModel,
    visibility: Signal<ChartVisibility>,
) -> impl IntoView {
    let render_model = model.clone();
    Effect::new(move |_| {
        let option = build_asset_chart_option(&render_model, visibility.get());
        render_chart(HOST_ID, &option);
    });
    view! {
        <EChartsHost
            id=HOST_ID
            label=format!("{} daily candlestick chart with volume and selectable technical indicators", model.symbol)
            class="min-h-[38rem] w-full flex-1 bg-canvas xl:min-h-0"
        />
    }
}

pub fn build_asset_chart_option(
    model: &AssetChartReadModel,
    visibility: ChartVisibility,
) -> String {
    let categories = model
        .candles
        .iter()
        .map(|candle| candle.timestamp.as_str())
        .collect::<Vec<_>>();
    let candles = model
        .candles
        .iter()
        .map(|candle| vec![candle.open, candle.close, candle.low, candle.high])
        .collect::<Vec<_>>();
    let volumes = model
        .candles
        .iter()
        .map(|candle| {
            json!({
                "value": candle.volume / 1_000_000.0,
                "itemStyle": { "color": if candle.close >= candle.open {
                    tokens::FINANCE_POSITIVE
                } else {
                    tokens::FINANCE_NEGATIVE
                }}
            })
        })
        .collect::<Vec<_>>();
    let mut series = vec![
        candlestick_series(candles, &model.price),
        volume_series(volumes),
    ];
    for (id, visible) in [
        ("ma-20", visibility.ma20),
        ("ma-50", visibility.ma50),
        ("ma-200", visibility.ma200),
    ] {
        if visible {
            if let Some(indicator) = indicator(model, id) {
                series.push(line_series(indicator, 0, 0, false));
            }
        }
    }
    if visibility.rsi {
        if let Some(indicator) = indicator(model, "rsi") {
            series.push(rsi_series(indicator));
        }
    }
    if visibility.macd {
        if let Some(indicator) = indicator(model, "macd") {
            series.extend(macd_series(indicator));
        }
    }
    let x_axes = (0..4)
        .map(|index| x_axis(&categories, index, index == 3))
        .collect::<Vec<_>>();
    json!({
        "animation": false,
        "backgroundColor": tokens::CANVAS,
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11 },
        "axisPointer": { "link": [{ "xAxisIndex": "all" }] },
        "tooltip": { "trigger": "axis", "axisPointer": { "type": "cross" } },
        "title": [
            panel_title("Volume", "47%"),
            panel_title("RSI (14)", "63%"),
            panel_title("MACD (12, 26, close)", "78%")
        ],
        "grid": [
            { "left": 12, "right": 62, "top": "2%", "height": "45%" },
            { "left": 12, "right": 62, "top": "49%", "height": "13%" },
            { "left": 12, "right": 62, "top": "65%", "height": "12%" },
            { "left": 12, "right": 62, "top": "80%", "height": "17%" }
        ],
        "xAxis": x_axes,
        "yAxis": [
            y_axis(0, None), y_axis(1, None), y_axis(2, Some((0.0, 100.0))), y_axis(3, None)
        ],
        "dataZoom": [{
            "type": "inside", "xAxisIndex": [0, 1, 2, 3], "start": 0, "end": 100,
            "filterMode": "none"
        }],
        "series": series
    })
    .to_string()
}

fn panel_title(text: &str, top: &str) -> Value {
    json!({
        "text": text,
        "left": 12,
        "top": top,
        "textStyle": { "color": tokens::TEXT_SECONDARY, "fontSize": 11, "fontWeight": 500 }
    })
}

fn x_axis(categories: &[&str], grid_index: usize, labels: bool) -> Value {
    json!({
        "type": "category",
        "gridIndex": grid_index,
        "data": categories,
        "boundaryGap": true,
        "axisLine": { "lineStyle": { "color": tokens::BORDER } },
        "axisTick": { "show": false },
        "axisLabel": { "show": labels, "color": tokens::TEXT_MUTED_READABLE, "interval": 19 },
        "splitLine": { "show": false },
        "min": "dataMin",
        "max": "dataMax"
    })
}

fn y_axis(grid_index: usize, range: Option<(f64, f64)>) -> Value {
    let mut axis = json!({
        "type": "value",
        "gridIndex": grid_index,
        "scale": true,
        "position": "right",
        "axisLine": { "show": false },
        "axisTick": { "show": false },
        "axisLabel": { "color": tokens::TEXT_MUTED_READABLE },
        "splitLine": {
            "show": true,
            "lineStyle": { "color": tokens::CHART_GRID, "type": "dashed" }
        }
    });
    if let Some((minimum, maximum)) = range {
        axis["min"] = json!(minimum);
        axis["max"] = json!(maximum);
    }
    axis
}

fn candlestick_series(data: Vec<Vec<f64>>, price: &str) -> Value {
    let last_price = price.parse::<f64>().unwrap_or(0.0);
    json!({
        "name": "Price",
        "type": "candlestick",
        "xAxisIndex": 0,
        "yAxisIndex": 0,
        "data": data,
        "itemStyle": {
            "color": tokens::FINANCE_POSITIVE,
            "color0": tokens::FINANCE_NEGATIVE,
            "borderColor": tokens::FINANCE_POSITIVE,
            "borderColor0": tokens::FINANCE_NEGATIVE
        },
        "markLine": {
            "symbol": "none",
            "silent": true,
            "label": {
                "show": true, "position": "end", "formatter": price,
                "color": tokens::TEXT_PRIMARY, "backgroundColor": tokens::FINANCE_POSITIVE,
                "padding": [3, 5]
            },
            "lineStyle": { "color": tokens::FINANCE_POSITIVE, "type": "dashed", "width": 1 },
            "data": [{ "yAxis": last_price }]
        }
    })
}

fn volume_series(data: Vec<Value>) -> Value {
    json!({
        "name": "Volume", "type": "bar", "xAxisIndex": 1, "yAxisIndex": 1,
        "data": data, "barMaxWidth": 8
    })
}

fn line_series(indicator: &ChartIndicator, x_axis: usize, y_axis: usize, smooth: bool) -> Value {
    let line = &indicator.lines[0];
    json!({
        "name": indicator.label,
        "type": "line",
        "xAxisIndex": x_axis,
        "yAxisIndex": y_axis,
        "data": line.values,
        "showSymbol": false,
        "smooth": smooth,
        "connectNulls": true,
        "lineStyle": { "color": tone_color(line.tone), "width": 1.25 },
        "itemStyle": { "color": tone_color(line.tone) }
    })
}

fn rsi_series(indicator: &ChartIndicator) -> Value {
    let mut series = line_series(indicator, 2, 2, true);
    series["markLine"] = json!({
        "symbol": "none",
        "silent": true,
        "label": { "show": false },
        "lineStyle": { "color": tokens::TEXT_MUTED_READABLE, "type": "dashed" },
        "data": [{ "yAxis": 30 }, { "yAxis": 70 }]
    });
    series
}

fn macd_series(indicator: &ChartIndicator) -> Vec<Value> {
    indicator
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 2 {
                let data = line
                    .values
                    .iter()
                    .map(|value| {
                        json!({
                            "value": value,
                            "itemStyle": { "color": if *value >= 0.0 {
                                tokens::FINANCE_POSITIVE
                            } else {
                                tokens::FINANCE_NEGATIVE
                            }}
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "name": "Histogram", "type": "bar", "xAxisIndex": 3, "yAxisIndex": 3,
                    "data": data, "barMaxWidth": 7
                })
            } else {
                json!({
                    "name": line.label, "type": "line", "xAxisIndex": 3, "yAxisIndex": 3,
                    "data": line.values, "showSymbol": false,
                    "lineStyle": { "color": tone_color(line.tone), "width": 1.2 }
                })
            }
        })
        .collect()
}

fn indicator<'a>(model: &'a AssetChartReadModel, id: &str) -> Option<&'a ChartIndicator> {
    model.indicators.iter().find(|indicator| indicator.id == id)
}

fn tone_color(tone: ChartTone) -> &'static str {
    match tone {
        ChartTone::Blue => tokens::INTERACTIVE_TEXT,
        ChartTone::Orange => tokens::LEVEL_SPECIAL,
        ChartTone::Purple => tokens::INTERACTIVE_SOURCE,
        ChartTone::Green => tokens::FINANCE_POSITIVE,
        ChartTone::Red => tokens::FINANCE_NEGATIVE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_and_multi_panel_axes_are_explicit() {
        let visibility = ChartVisibility {
            ma20: true,
            ma50: false,
            ma200: false,
            rsi: true,
            macd: true,
        };
        assert!(visibility.ma20);
        assert!(!visibility.ma50);
        assert_eq!(x_axis(&["May 21"], 3, true)["gridIndex"], 3);
        assert_eq!(y_axis(2, Some((0.0, 100.0)))["max"], 100.0);
    }
}
