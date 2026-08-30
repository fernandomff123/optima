use crate::{application::asset_volatility::VolatilityHistoryPoint, design_system::tokens};
use leptos::prelude::*;

const HOST_ID: &str = "asset-volatility-history";

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderVolatilityHistory(id, historyJson, themeText) {
  const [canvas,surface,grid,border,text,muted,purple,cyan,blue] = themeText.split('\u001f');
  const history = JSON.parse(historyJson), x = history.map(point => point.label);
  const line = (name,key,color) => ({type:'scatter',mode:'lines',name,x,y:history.map(point => point[key]),line:{color,width:1.8},hovertemplate:`${name}<br>%{x}<br>%{y:.1f}%<extra></extra>`});
  const earnings = history.flatMap((point,index) => point.earnings ? [{type:'line',xref:'x',x0:x[index],x1:x[index],yref:'paper',y0:0,y1:1,line:{color:muted,width:1,dash:'dot'}}] : []);
  const axis = {gridcolor:grid,linecolor:border,zeroline:false,tickfont:{color:muted,size:10}};
  Plotly.react(id,[line('ATM IV 30D','atm',purple),line('RV20','rv20',cyan),line('RV60','rv60',blue)],
    {paper_bgcolor:surface,plot_bgcolor:canvas,font:{color:text,size:10},margin:{l:48,r:18,t:28,b:42},legend:{orientation:'h',x:0.2,y:1.15},shapes:earnings,xaxis:{...axis},yaxis:{...axis,title:'Volatility (%)'}},
    {responsive:true,displaylogo:false,displayModeBar:false,scrollZoom:false});
}
export function purgeVolatilityHistory(id) { Plotly.purge(id); }
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderVolatilityHistory)]
    fn render_plot(id: &str, history: &str, theme: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeVolatilityHistory)]
    fn purge_plot(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_plot(_id: &str, _history: &str, _theme: &str) {}
#[cfg(not(target_arch = "wasm32"))]
fn purge_plot(_id: &str) {}

#[component]
pub fn VolatilityHistoryChart(history: Vec<VolatilityHistoryPoint>) -> impl IntoView {
    let history_json = serde_json::to_string(&history.iter().map(|point| serde_json::json!({"label":point.label,"atm":point.atm_iv_30d_percent,"rv20":point.realized_volatility_20d_percent,"rv60":point.realized_volatility_60d_percent,"earnings":point.earnings})).collect::<Vec<_>>()).expect("history fixture serializes");
    let theme = [
        tokens::CANVAS,
        tokens::SURFACE,
        tokens::CHART_GRID,
        tokens::BORDER,
        tokens::TEXT_PRIMARY,
        tokens::TEXT_MUTED_READABLE,
        tokens::STATE_FOCUS,
        tokens::FINANCE_POSITIVE,
        tokens::INTERACTIVE_TEXT,
    ]
    .join("\u{001f}");
    Effect::new(move |_| render_plot(HOST_ID, &history_json, &theme));
    on_cleanup(move || purge_plot(HOST_ID));
    view! { <section class="flex h-full min-h-64 flex-col border border-border bg-surface" aria-label="Historical implied and realized volatility"><div class="panel-header"><h2 class="text-sm font-semibold">"Historical IV vs Realized Volatility"</h2><span class="text-[0.6875rem] text-text-secondary">"Earnings marked by dotted lines"</span></div><div id=HOST_ID class="min-h-56 flex-1 bg-canvas" role="img" aria-label="Historical ATM implied volatility and realized volatility"></div></section> }
}
