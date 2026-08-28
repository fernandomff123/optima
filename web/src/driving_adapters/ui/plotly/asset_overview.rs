use crate::{application::asset_overview::PriceVolumeChart, design_system::tokens};
use leptos::prelude::*;

const HOST_ID: &str = "asset-overview-price-volume";

#[derive(Clone, Debug, PartialEq)]
pub struct PlotlySpec {
    times: String,
    tick_values: String,
    prices: Vec<f64>,
    volumes: Vec<f64>,
}

pub fn build_price_volume_plot(chart: &PriceVolumeChart) -> PlotlySpec {
    let tick_values = chart
        .times
        .iter()
        .enumerate()
        .filter(|(index, _)| index % 6 == 0 || *index + 1 == chart.times.len())
        .map(|(_, time)| time.as_str())
        .collect::<Vec<_>>()
        .join("\u{001f}");
    PlotlySpec {
        times: chart.times.join("\u{001f}"),
        tick_values,
        prices: chart.prices.clone(),
        volumes: chart.volumes.clone(),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderOverviewPlot(id, timesText, ticksText, prices, volumes, themeText) {
  const times = timesText.split('\u001f');
  const tickValues = ticksText.split('\u001f');
  const [canvas, surface, grid, border, text, muted, blue, green, red] = themeText.split('\u001f');
  const priceValues = Array.from(prices);
  const volumeColors = priceValues.map((price, index) => index === 0 || price >= priceValues[index - 1] ? green : red);
  const data = [
    {type:'scatter', mode:'lines', name:'Price', x:times, y:priceValues,
     line:{color:blue,width:1.5}, fill:'tozeroy', fillcolor:'rgba(27,93,202,0.10)',
     hovertemplate:'%{x}<br>Price %{y:,.2f}<extra></extra>'},
    {type:'bar', name:'Volume', x:times, y:Array.from(volumes), yaxis:'y2', opacity:0.52,
     marker:{color:volumeColors}, hovertemplate:'%{x}<br>Volume %{y:.2f}M<extra></extra>'}
  ];
  const axis = {gridcolor:grid, linecolor:border, tickfont:{color:muted}};
  const layout = {paper_bgcolor:surface, plot_bgcolor:canvas, font:{color:text,size:11},
    showlegend:false, bargap:0.18, margin:{l:44,r:44,t:6,b:30},
    xaxis:{...axis,tickmode:'array',tickvals:tickValues,ticktext:tickValues},
    yaxis:{...axis,domain:[0.23,1],range:[5250,5325]}, yaxis2:{...axis,domain:[0,0.17]}};
  Plotly.react(id, data, layout, {responsive:true,displaylogo:false,modeBarButtonsToRemove:['lasso2d','select2d']});
}
export function purgeOverviewPlot(id) { Plotly.purge(id); }
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderOverviewPlot)]
    fn render_plot(
        id: &str,
        times: &str,
        tick_values: &str,
        prices: &[f64],
        volumes: &[f64],
        theme: &str,
    );
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeOverviewPlot)]
    fn purge_plot(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_plot(
    _id: &str,
    _times: &str,
    _tick_values: &str,
    _prices: &[f64],
    _volumes: &[f64],
    _theme: &str,
) {
}
#[cfg(not(target_arch = "wasm32"))]
fn purge_plot(_id: &str) {}

#[component]
pub fn AssetOverviewChart(chart: PriceVolumeChart) -> impl IntoView {
    let description = chart.description.clone();
    let spec = build_price_volume_plot(&chart);
    let theme = [
        tokens::CANVAS,
        tokens::SURFACE,
        tokens::CHART_GRID,
        tokens::BORDER,
        tokens::TEXT_SECONDARY,
        tokens::TEXT_MUTED_READABLE,
        tokens::INTERACTIVE_TEXT,
        tokens::FINANCE_POSITIVE,
        tokens::FINANCE_NEGATIVE,
    ]
    .join("\u{001f}");
    Effect::new(move |_| {
        render_plot(
            HOST_ID,
            &spec.times,
            &spec.tick_values,
            &spec.prices,
            &spec.volumes,
            &theme,
        )
    });
    on_cleanup(move || purge_plot(HOST_ID));
    view! { <div><div id=HOST_ID class="h-72 min-h-72 w-full bg-canvas sm:h-[20.5rem]" role="img" aria-label=description.clone()></div><p class="sr-only">{description}</p></div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_maps_provider_neutral_chart_without_plotly_types() {
        let chart = PriceVolumeChart {
            times: vec!["09:30".into()],
            prices: vec![5303.27],
            volumes: vec![0.8421],
            price_unit: "USD".into(),
            volume_unit: "contracts".into(),
            description: "Price and volume".into(),
        };
        let spec = build_price_volume_plot(&chart);
        assert_eq!(spec.times, "09:30");
        assert_eq!(spec.tick_values, "09:30");
        assert_eq!(spec.prices, vec![5303.27]);
        assert_eq!(spec.volumes, vec![0.8421]);
    }
}
