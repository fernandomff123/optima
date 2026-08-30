use crate::{application::asset_volatility::VolatilityGrid, design_system::tokens};
use leptos::prelude::*;

const HOST_ID: &str = "asset-volatility-surface";

#[derive(Clone, Debug, PartialEq)]
pub struct VolatilitySurfaceSpec {
    moneyness: Vec<f64>,
    days_to_expiry: Vec<f64>,
    values: Vec<f64>,
}

pub fn build_surface_plot(grid: &VolatilityGrid) -> VolatilitySurfaceSpec {
    VolatilitySurfaceSpec {
        moneyness: grid.moneyness.clone(),
        days_to_expiry: grid
            .days_to_expiry
            .iter()
            .map(|value| f64::from(*value))
            .collect(),
        values: grid
            .implied_volatility_percent
            .iter()
            .flatten()
            .copied()
            .collect(),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function renderVolatilitySurface(id, moneynessValues, expiryValues, flatValues, themeText) {
  const [canvas, surface, grid, border, text, muted, blue, purple] = themeText.split('\u001f');
  const x = Array.from(moneynessValues);
  const y = Array.from(expiryValues);
  const flat = Array.from(flatValues);
  const z = y.map((_, column) => x.map((_, row) => flat[row * y.length + column]));
  const observedX = [], observedY = [], observedZ = [];
  y.forEach((expiry, column) => x.forEach((money, row) => {
    observedX.push(money); observedY.push(expiry); observedZ.push(z[column][row]);
  }));
  const data = [
    {type:'surface',x,y,z,name:'IV Surface',showscale:false,opacity:0.92,
     colorscale:[[0,blue],[0.55,'#1B5DCA'],[1,purple]],
     contours:{z:{show:true,usecolormap:true,highlightcolor:text,project:{z:true}}},
     hovertemplate:'Moneyness %{x:.2f}<br>%{y:.0f} DTE<br>IV %{z:.1f}%<extra></extra>'},
    {type:'scatter3d',mode:'markers',name:'Observed IV',x:observedX,y:observedY,z:observedZ,
     marker:{color:text,size:2.5,opacity:0.9},hoverinfo:'skip'}
  ];
  const axis = {gridcolor:grid,linecolor:border,zerolinecolor:border,tickfont:{color:muted,size:10},titlefont:{color:text,size:11}};
  const layout = {paper_bgcolor:surface,plot_bgcolor:canvas,font:{color:text,size:11},showlegend:false,
    margin:{l:0,r:0,t:8,b:0},scene:{bgcolor:canvas,dragmode:false,aspectratio:{x:1.35,y:1.15,z:0.72},camera:{eye:{x:1.65,y:-1.85,z:0.9},center:{x:0,y:0,z:-0.08},up:{x:0,y:0,z:1}},
      xaxis:{...axis,title:'Moneyness'},yaxis:{...axis,title:'Days to Expiry'},zaxis:{...axis,title:'Implied Volatility (%)'}}};
  Plotly.react(id,data,layout,{responsive:true,displaylogo:false,displayModeBar:false,scrollZoom:false,doubleClick:false});
}
export function purgeVolatilitySurface(id) { Plotly.purge(id); }
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = renderVolatilitySurface)]
    fn render_plot(id: &str, moneyness: &[f64], days: &[f64], values: &[f64], theme: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = purgeVolatilitySurface)]
    fn purge_plot(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_plot(_id: &str, _moneyness: &[f64], _days: &[f64], _values: &[f64], _theme: &str) {}
#[cfg(not(target_arch = "wasm32"))]
fn purge_plot(_id: &str) {}

#[component]
pub fn VolatilitySurfaceChart(grid: VolatilityGrid) -> impl IntoView {
    let spec = build_surface_plot(&grid);
    let theme = [
        tokens::CANVAS,
        tokens::SURFACE,
        tokens::CHART_GRID,
        tokens::BORDER,
        tokens::TEXT_PRIMARY,
        tokens::TEXT_MUTED_READABLE,
        tokens::INTERACTIVE_SOURCE,
        tokens::STATE_FOCUS,
    ]
    .join("\u{001f}");
    Effect::new(move |_| {
        render_plot(
            HOST_ID,
            &spec.moneyness,
            &spec.days_to_expiry,
            &spec.values,
            &theme,
        )
    });
    on_cleanup(move || purge_plot(HOST_ID));
    view! { <div id=HOST_ID class="min-h-[25rem] w-full flex-1 bg-canvas" role="img" tabindex="0" aria-label="Illustrative AAPL implied volatility surface by moneyness and days to expiry"></div> }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_builder_preserves_the_provider_neutral_grid() {
        let grid = VolatilityGrid {
            moneyness: vec![0.9, 1.0],
            days_to_expiry: vec![7, 30],
            implied_volatility_percent: vec![vec![25.0, 24.0], vec![23.0, 22.0]],
            selected_moneyness_index: 1,
            selected_expiry_index: 1,
        };
        let spec = build_surface_plot(&grid);
        assert_eq!(spec.values, vec![25.0, 24.0, 23.0, 22.0]);
        assert_eq!(spec.days_to_expiry, vec![7.0, 30.0]);
    }
}
