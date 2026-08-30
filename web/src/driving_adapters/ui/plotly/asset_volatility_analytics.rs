use crate::{
    application::asset_volatility::{VolatilitySmile, VolatilityTermPoint},
    design_system::tokens,
};
use leptos::prelude::*;

const SMILE_HOST_ID: &str = "asset-volatility-smiles";
const TERM_HOST_ID: &str = "asset-volatility-term";

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderVolatilityAnalytics(smileId, termId, moneynessValues, smileJson, termJson, themeText) {
  const [canvas,surface,grid,border,text,muted,blue,purple,cyan,pink] = themeText.split('\u001f');
  const x = Array.from(moneynessValues), smiles = JSON.parse(smileJson), term = JSON.parse(termJson);
  const colors = [blue,purple,cyan,pink];
  const axis = {gridcolor:grid,linecolor:border,zeroline:false,tickfont:{color:muted,size:10}};
  const base = {paper_bgcolor:surface,plot_bgcolor:canvas,font:{color:text,size:10},margin:{l:48,r:16,t:24,b:42}};
  Plotly.react(smileId, smiles.map((curve,index) => ({type:'scatter',mode:'lines+markers',name:curve.label,x,y:curve.values,line:{color:colors[index],width:1.7},marker:{size:4},hovertemplate:`${curve.label}<br>Moneyness %{x:.2f}<br>IV %{y:.1f}%<extra></extra>`})),
    {...base,legend:{orientation:'h',x:0.55,y:1.17},xaxis:{...axis,title:'Moneyness'},yaxis:{...axis,title:'Implied Volatility (%)'}},
    {responsive:true,displaylogo:false,displayModeBar:false});
  Plotly.react(termId,[{type:'scatter',mode:'lines+markers',x:term.map(p=>p.days),y:term.map(p=>p.value),line:{color:blue,width:2},marker:{color:blue,size:6},hovertemplate:'%{x:.0f} DTE<br>ATM IV %{y:.1f}%<extra></extra>'}],
    {...base,showlegend:false,xaxis:{...axis,title:'Days to Expiry'},yaxis:{...axis,title:'ATM IV (%)'}},
    {responsive:true,displaylogo:false,displayModeBar:false});
}
export function purgeVolatilityAnalytics(smileId, termId) { Plotly.purge(smileId); Plotly.purge(termId); }
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderVolatilityAnalytics)]
    fn render_plots(
        smile_id: &str,
        term_id: &str,
        moneyness: &[f64],
        smiles: &str,
        term: &str,
        theme: &str,
    );
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeVolatilityAnalytics)]
    fn purge_plots(smile_id: &str, term_id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_plots(
    _smile_id: &str,
    _term_id: &str,
    _moneyness: &[f64],
    _smiles: &str,
    _term: &str,
    _theme: &str,
) {
}
#[cfg(not(target_arch = "wasm32"))]
fn purge_plots(_smile_id: &str, _term_id: &str) {}

#[component]
pub fn VolatilityAnalytics(
    moneyness: Vec<f64>,
    smiles: Vec<VolatilitySmile>,
    term_structure: Vec<VolatilityTermPoint>,
) -> impl IntoView {
    let smile_json = serde_json::to_string(&smiles.iter().map(|curve| serde_json::json!({"label":curve.label,"values":curve.implied_volatility_percent})).collect::<Vec<_>>()).expect("smile fixture serializes");
    let term_json = serde_json::to_string(&term_structure.iter().map(|point| serde_json::json!({"days":point.days_to_expiry,"value":point.implied_volatility_percent})).collect::<Vec<_>>()).expect("term fixture serializes");
    let theme = [
        tokens::CANVAS,
        tokens::SURFACE,
        tokens::CHART_GRID,
        tokens::BORDER,
        tokens::TEXT_PRIMARY,
        tokens::TEXT_MUTED_READABLE,
        tokens::INTERACTIVE_TEXT,
        tokens::STATE_FOCUS,
        tokens::FINANCE_POSITIVE,
        tokens::LEVEL_SPECIAL,
    ]
    .join("\u{001f}");
    Effect::new(move |_| {
        render_plots(
            SMILE_HOST_ID,
            TERM_HOST_ID,
            &moneyness,
            &smile_json,
            &term_json,
            &theme,
        )
    });
    on_cleanup(move || purge_plots(SMILE_HOST_ID, TERM_HOST_ID));
    view! {
        <section class="grid min-h-0 border border-border bg-surface xl:grid-cols-2" aria-label="Coordinated volatility views">
            <div class="flex min-h-64 flex-col border-b border-border xl:min-h-0 xl:border-b-0 xl:border-r"><div class="panel-header"><h2 class="text-sm font-semibold">"IV Smile by Expiry (Calls + Puts)"</h2></div><div id=SMILE_HOST_ID class="min-h-56 flex-1 bg-canvas" role="img" aria-label="Implied volatility smiles for selected expiries"></div></div>
            <div class="flex min-h-64 flex-col xl:min-h-0"><div class="panel-header"><h2 class="text-sm font-semibold">"ATM IV Term Structure"</h2></div><div id=TERM_HOST_ID class="min-h-56 flex-1 bg-canvas" role="img" aria-label="ATM implied volatility term structure"></div></div>
        </section>
    }
}
