use crate::{application::asset_options::VolatilitySmile, design_system::tokens};
use leptos::prelude::*;

const HOST_ID: &str = "asset-options-iv-smile";

#[derive(Clone, Debug, PartialEq)]
pub struct SmilePlotSpec {
    strikes: Vec<f64>,
    call_iv: Vec<f64>,
    put_iv: Vec<f64>,
    spot: f64,
    spot_label: String,
}

pub fn build_smile_plot(smile: &VolatilitySmile) -> SmilePlotSpec {
    SmilePlotSpec {
        strikes: smile.strikes.clone(),
        call_iv: smile.call_iv.clone(),
        put_iv: smile.put_iv.clone(),
        spot: smile.spot,
        spot_label: smile.spot_label.clone(),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderOptionsSmile(id, strikes, callIv, putIv, spot, spotLabel, themeText) {
  const [canvas, surface, grid, border, text, muted, blue, red, orange] = themeText.split('\u001f');
  const x = Array.from(strikes);
  const traces = [
    {type:'scatter', mode:'lines+markers', name:'Call IV', x, y:Array.from(callIv),
     line:{color:blue,width:1.8}, marker:{color:blue,size:5}, hovertemplate:'Strike %{x:.2f}<br>Call IV %{y:.1f}%<extra></extra>'},
    {type:'scatter', mode:'lines+markers', name:'Put IV', x, y:Array.from(putIv),
     line:{color:red,width:1.8}, marker:{color:red,size:5}, hovertemplate:'Strike %{x:.2f}<br>Put IV %{y:.1f}%<extra></extra>'}
  ];
  const axis = {gridcolor:grid,gridwidth:1,linecolor:border,tickfont:{color:muted,size:11},zeroline:false};
  const layout = {paper_bgcolor:surface,plot_bgcolor:canvas,font:{color:text,size:11},
    margin:{l:52,r:18,t:14,b:44},showlegend:true,
    legend:{orientation:'h',x:0,y:1.12,font:{color:text,size:11}},
    xaxis:{...axis,title:{text:'Strike',font:{color:text,size:11}}},
    yaxis:{...axis,title:{text:'Implied Volatility (%)',font:{color:text,size:11}},range:[15,30]},
    shapes:[{type:'line',x0:spot,x1:spot,yref:'paper',y0:0,y1:1,line:{color:orange,width:1,dash:'dot'}}],
    annotations:[{x:spot,y:1.01,yref:'paper',text:spotLabel,showarrow:false,font:{color:orange,size:11},yanchor:'bottom'}]
  };
  Plotly.react(id,traces,layout,{responsive:true,displaylogo:false,displayModeBar:false,scrollZoom:false});
}
export function purgeOptionsSmile(id) { Plotly.purge(id); }
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderOptionsSmile)]
    fn render_plot(
        id: &str,
        strikes: &[f64],
        call_iv: &[f64],
        put_iv: &[f64],
        spot: f64,
        spot_label: &str,
        theme: &str,
    );
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeOptionsSmile)]
    fn purge_plot(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_plot(
    _id: &str,
    _strikes: &[f64],
    _call_iv: &[f64],
    _put_iv: &[f64],
    _spot: f64,
    _spot_label: &str,
    _theme: &str,
) {
}
#[cfg(not(target_arch = "wasm32"))]
fn purge_plot(_id: &str) {}

#[component]
pub fn OptionsSmileChart(smile: VolatilitySmile) -> impl IntoView {
    let description = smile.description.clone();
    let spec = build_smile_plot(&smile);
    let theme = [
        tokens::CANVAS,
        tokens::SURFACE,
        tokens::CHART_GRID,
        tokens::BORDER,
        tokens::TEXT_SECONDARY,
        tokens::TEXT_MUTED_READABLE,
        tokens::INTERACTIVE_TEXT,
        tokens::FINANCE_NEGATIVE,
        tokens::LEVEL_SPECIAL,
    ]
    .join("\u{001f}");
    Effect::new(move |_| {
        render_plot(
            HOST_ID,
            &spec.strikes,
            &spec.call_iv,
            &spec.put_iv,
            spec.spot,
            &spec.spot_label,
            &theme,
        )
    });
    on_cleanup(move || purge_plot(HOST_ID));
    view! {
        <div class="flex min-h-0 flex-1 flex-col">
            <div class="flex h-11 shrink-0 items-center gap-6 border-b border-border px-4 text-xs font-medium"><span class="flex h-11 items-center border-b-2 border-interactive-text text-text-primary">"IV Smile"</span>{["Open Interest", "Volume", "Put/Call"].into_iter().map(|label| view! { <button class="cursor-not-allowed text-text-secondary opacity-60" type="button" disabled>{label}</button> }).collect_view()}</div>
            <div id=HOST_ID class="min-h-64 w-full flex-1 bg-canvas xl:min-h-0" role="img" tabindex="0" aria-label=description.clone()></div><p class="sr-only">{description}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_preserves_provider_neutral_smile_values() {
        let smile = VolatilitySmile {
            strikes: vec![190.0],
            call_iv: vec![20.1],
            put_iv: vec![19.4],
            spot: 191.13,
            spot_label: "191.13".into(),
            description: "Mock smile".into(),
        };
        let spec = build_smile_plot(&smile);
        assert_eq!(spec.strikes, vec![190.0]);
        assert_eq!(spec.call_iv, vec![20.1]);
        assert_eq!(spec.put_iv, vec![19.4]);
        assert_eq!(spec.spot, 191.13);
    }
}
