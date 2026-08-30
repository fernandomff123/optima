use crate::{
    application::asset_chart::{ChartIndicator, ChartTone, GexLevel},
    design_system::tokens,
};
use serde_json::{Value, json};

pub(super) fn candlestick_series(
    data: Vec<Vec<f64>>,
    price: &str,
    gex_levels: &[GexLevel],
) -> Value {
    let last_price = price.parse::<f64>().unwrap_or(0.0);
    let mut levels = vec![json!({
        "yAxis": last_price,
        "label": { "formatter": price, "backgroundColor": tokens::FINANCE_POSITIVE },
        "lineStyle": { "color": tokens::FINANCE_POSITIVE }
    })];
    levels.extend(gex_levels.iter().map(|level| {
        let color = tone_color(level.tone);
        json!({
            "name": level.label,
            "yAxis": level.value,
            "label": {
                "formatter": format!("{} {:.2}", level.label, level.value),
                "backgroundColor": tokens::SURFACE_ELEVATED,
                "color": color
            },
            "lineStyle": { "color": color, "type": "dashed", "width": 1 }
        })
    }));
    json!({
        "name": "Price", "type": "candlestick", "xAxisIndex": 0, "yAxisIndex": 0,
        "data": data,
        "itemStyle": {
            "color": tokens::FINANCE_POSITIVE,
            "color0": tokens::FINANCE_NEGATIVE,
            "borderColor": tokens::FINANCE_POSITIVE,
            "borderColor0": tokens::FINANCE_NEGATIVE
        },
        "markLine": {
            "symbol": "none", "silent": true,
            "label": {
                "show": true, "position": "end", "color": tokens::TEXT_PRIMARY,
                "padding": [3, 5]
            },
            "lineStyle": { "type": "dashed", "width": 1 },
            "data": levels
        }
    })
}

pub(super) fn volume_series(data: Vec<Value>) -> Value {
    json!({
        "name": "Volume", "type": "bar", "xAxisIndex": 1, "yAxisIndex": 1,
        "data": data, "barMaxWidth": 8
    })
}

pub(super) fn line_series(
    indicator: &ChartIndicator,
    x_axis: usize,
    y_axis: usize,
    smooth: bool,
) -> Value {
    let line = &indicator.lines[0];
    json!({
        "name": indicator.label, "type": "line", "xAxisIndex": x_axis,
        "yAxisIndex": y_axis, "data": line.values, "showSymbol": false,
        "smooth": smooth, "connectNulls": true,
        "lineStyle": { "color": tone_color(line.tone), "width": 1.25 },
        "itemStyle": { "color": tone_color(line.tone) }
    })
}

pub(super) fn multi_line_series(
    indicator: &ChartIndicator,
    x_axis: usize,
    y_axis: usize,
) -> Vec<Value> {
    indicator
        .lines
        .iter()
        .map(|line| {
            json!({
                "name": format!("{} · {}", indicator.label, line.label),
                "type": "line", "xAxisIndex": x_axis, "yAxisIndex": y_axis,
                "data": line.values, "showSymbol": false, "connectNulls": true,
                "lineStyle": {
                    "color": tone_color(line.tone), "width": 1,
                    "type": "dashed", "opacity": 0.72
                }
            })
        })
        .collect()
}

pub(super) fn rsi_series(indicator: &ChartIndicator) -> Value {
    let mut series = line_series(indicator, 2, 2, true);
    series["markLine"] = json!({
        "symbol": "none", "silent": true, "label": { "show": false },
        "lineStyle": { "color": tokens::TEXT_MUTED_READABLE, "type": "dashed" },
        "data": [{ "yAxis": 30 }, { "yAxis": 70 }]
    });
    series
}

pub(super) fn macd_series(indicator: &ChartIndicator) -> Vec<Value> {
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
                    "name": "Histogram", "type": "bar", "xAxisIndex": 3,
                    "yAxisIndex": 3, "data": data, "barMaxWidth": 7
                })
            } else {
                json!({
                    "name": line.label, "type": "line", "xAxisIndex": 3,
                    "yAxisIndex": 3, "data": line.values, "showSymbol": false,
                    "lineStyle": { "color": tone_color(line.tone), "width": 1.2 }
                })
            }
        })
        .collect()
}

pub(super) fn tone_color(tone: ChartTone) -> &'static str {
    match tone {
        ChartTone::Blue => tokens::INTERACTIVE_TEXT,
        ChartTone::Orange => tokens::LEVEL_SPECIAL,
        ChartTone::Purple => tokens::INTERACTIVE_SOURCE,
        ChartTone::Green => tokens::FINANCE_POSITIVE,
        ChartTone::Red => tokens::FINANCE_NEGATIVE,
    }
}
