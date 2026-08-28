use crate::{application::asset_overview::PriceVolumeChart, design_system::tokens};
use leptos::prelude::*;

const HOST_ID: &str = "asset-overview-price-volume";

#[derive(Clone, Debug, PartialEq)]
pub struct PlotlySpec {
    times: String,
    tick_values: String,
    prices: Vec<f64>,
    volumes: Vec<f64>,
    session_end: String,
    last_price: String,
    last_volume: String,
}

pub fn build_price_volume_plot(chart: &PriceVolumeChart) -> PlotlySpec {
    let tick_values = [
        "09:30", "10:00", "10:30", "11:00", "11:30", "12:00", "12:30", "13:00", "13:30", "14:00",
        "14:30", "15:00", "15:30", "16:00",
    ]
    .join("\u{001f}");
    PlotlySpec {
        times: chart.times.join("\u{001f}"),
        tick_values,
        prices: chart.prices.clone(),
        volumes: chart.volumes.clone(),
        session_end: chart.session_end.clone(),
        last_price: chart.last_price.clone(),
        last_volume: chart.last_volume.clone(),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderOverviewPlot(id, timesText, ticksText, prices, volumes, sessionEnd, lastPriceText, lastVolumeText, themeText) {
  const times = timesText.split('\u001f');
  const tickValues = ticksText.split('\u001f');
  const minuteOfDay = time => { const [hour, minute] = time.split(':').map(Number); return hour * 60 + minute; };
  const minutes = times.map(minuteOfDay);
  const tickMinutes = tickValues.map(minuteOfDay);
  const [canvas, surface, grid, border, text, muted, blue, green, red] = themeText.split('\u001f');
  const priceValues = Array.from(prices);
  const volumeColors = priceValues.map((price, index) => index === 0 || price >= priceValues[index - 1] ? green : red);
  const data = [
    {type:'scatter', mode:'lines', name:'Price', x:minutes, y:priceValues, customdata:times,
     line:{color:blue,width:1.5}, fill:'tozeroy', fillcolor:'rgba(27,93,202,0.10)',
     hovertemplate:'%{customdata}<br>Price %{y:,.2f}<extra></extra>'},
    {type:'bar', name:'Volume', x:minutes, y:Array.from(volumes), customdata:times, yaxis:'y2', opacity:0.52,
     marker:{color:volumeColors}, hovertemplate:'%{customdata}<br>Volume %{y:.2f}M<extra></extra>'}
  ];
  const lastPrice = priceValues[priceValues.length - 1];
  const lastVolume = Array.from(volumes).at(-1);
  const priceMin = Math.min(...priceValues);
  const priceMax = Math.max(...priceValues);
  const pricePadding = Math.max((priceMax - priceMin) * 0.12, 0.25);
  const axis = {gridcolor:grid,gridwidth:1,linecolor:border,tickfont:{color:muted,size:10},zeroline:false};
  const layout = {paper_bgcolor:surface, plot_bgcolor:canvas, font:{color:text,size:11},
    showlegend:false, bargap:0.16, margin:{l:8,r:50,t:4,b:28},
    xaxis:{...axis,range:[minuteOfDay('09:30'),minuteOfDay(sessionEnd)],tickmode:'array',tickvals:tickMinutes,ticktext:tickValues},
    yaxis:{...axis,side:'right',domain:[0.23,1],range:[priceMin-pricePadding,priceMax+pricePadding],automargin:false}, yaxis2:{...axis,side:'right',domain:[0,0.17],tickformat:'.1s'},
    shapes:[{type:'line',xref:'paper',x0:0,x1:1,yref:'y',y0:lastPrice,y1:lastPrice,line:{color:blue,width:1,dash:'dot'}}],
    annotations:[
      {xref:'paper',x:1.006,yref:'y',y:lastPrice,text:lastPriceText,showarrow:false,xanchor:'left',font:{color:text,size:10},bgcolor:blue,borderpad:3},
      {xref:'paper',x:1.006,yref:'y2',y:lastVolume,text:lastVolumeText,showarrow:false,xanchor:'left',font:{color:text,size:9},bgcolor:green,borderpad:2}
    ]};
  Plotly.react(id, data, layout, {responsive:true,displaylogo:false,displayModeBar:false});
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
        session_end: &str,
        last_price: &str,
        last_volume: &str,
        theme: &str,
    );
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeOverviewPlot)]
    fn purge_plot(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn render_plot(
    _id: &str,
    _times: &str,
    _tick_values: &str,
    _prices: &[f64],
    _volumes: &[f64],
    _session_end: &str,
    _last_price: &str,
    _last_volume: &str,
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
            &spec.session_end,
            &spec.last_price,
            &spec.last_volume,
            &theme,
        )
    });
    on_cleanup(move || purge_plot(HOST_ID));
    let periods = ["1D", "5D", "1M", "3M", "6M", "YTD", "1Y", "5Y", "MAX"];
    view! { <div><div id=HOST_ID class="h-72 min-h-72 w-full bg-canvas sm:h-[18rem]" role="img" aria-label=description.clone()></div><p class="sr-only">{description}</p><div class="dense-scrollbar overflow-x-auto border-t border-border"><div class="flex min-w-max items-center gap-6 px-4 text-xs font-medium" aria-label="Chart period"><span class="border-b-2 border-interactive-text py-2 text-interactive-text" aria-current="true">"1D"</span>{periods.into_iter().skip(1).map(|period| view! { <span class="py-2 text-text-secondary opacity-70" aria-disabled="true">{period}</span> }).collect_view()}</div></div></div> }
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
            session_end: "16:00".into(),
            last_price: "5,303.27".into(),
            last_volume: "842.1K".into(),
            description: "Price and volume".into(),
        };
        let spec = build_price_volume_plot(&chart);
        assert_eq!(spec.times, "09:30");
        assert!(spec.tick_values.starts_with("09:30"));
        assert!(spec.tick_values.ends_with("16:00"));
        assert_eq!(spec.prices, vec![5303.27]);
        assert_eq!(spec.volumes, vec![0.8421]);
        assert_eq!(spec.session_end, "16:00");
        assert_eq!(spec.last_price, "5,303.27");
    }
}
