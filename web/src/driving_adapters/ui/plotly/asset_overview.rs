use crate::{application::asset_overview::PriceVolumeChart, design_system::tokens};
use leptos::prelude::*;

const HOST_ID: &str = "asset-overview-price-volume";

#[derive(Clone, Debug, PartialEq)]
pub struct PlotlySpec {
    times: String,
    prices: Vec<f64>,
    volumes: Vec<f64>,
}

pub fn build_price_volume_plot(chart: &PriceVolumeChart) -> PlotlySpec {
    PlotlySpec {
        times: chart.times.join("\u{001f}"),
        prices: chart.prices.clone(),
        volumes: chart.volumes.clone(),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderOverviewPlot(id, timesText, prices, volumes, themeText) {
  const times = timesText.split('\u001f');
  const [canvas, surface, grid, border, text, muted, blue, green] = themeText.split('\u001f');
  const data = [
    {type:'scatter', mode:'lines', name:'Price', x:times, y:Array.from(prices),
     line:{color:blue,width:1.7}, fill:'tozeroy', fillcolor:'rgba(27,93,202,0.16)',
     hovertemplate:'%{x}<br>Price %{y:,.2f}<extra></extra>'},
    {type:'bar', name:'Volume', x:times, y:Array.from(volumes), yaxis:'y2', opacity:0.62,
     marker:{color:green}, hovertemplate:'%{x}<br>Volume %{y:.2f}M<extra></extra>'}
  ];
  const axis = {gridcolor:grid, linecolor:border, tickfont:{color:muted}};
  const layout = {paper_bgcolor:surface, plot_bgcolor:canvas, font:{color:text,size:11},
    showlegend:false, margin:{l:48,r:48,t:10,b:36}, xaxis:{...axis},
    yaxis:{...axis,domain:[0.25,1],range:[5250,5325]}, yaxis2:{...axis,domain:[0,0.2]}};
  Plotly.react(id, data, layout, {responsive:true,displaylogo:false,modeBarButtonsToRemove:['lasso2d','select2d']});
}
export function purgeOverviewPlot(id) { Plotly.purge(id); }
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderOverviewPlot)]
    fn render_plot(id: &str, times: &str, prices: &[f64], volumes: &[f64], theme: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeOverviewPlot)]
    fn purge_plot(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_plot(_id: &str, _times: &str, _prices: &[f64], _volumes: &[f64], _theme: &str) {}
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
    ]
    .join("\u{001f}");
    Effect::new(move |_| render_plot(HOST_ID, &spec.times, &spec.prices, &spec.volumes, &theme));
    on_cleanup(move || purge_plot(HOST_ID));
    view! { <div><div id=HOST_ID class="h-[22rem] min-h-72 w-full bg-canvas" role="img" aria-label=description.clone()></div><p class="sr-only">{description}</p></div> }
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
        assert_eq!(spec.prices, vec![5303.27]);
        assert_eq!(spec.volumes, vec![0.8421]);
    }
}
