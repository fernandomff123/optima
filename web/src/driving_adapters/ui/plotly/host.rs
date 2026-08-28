use super::PlotlyTheme;
use leptos::prelude::*;

const HOST_ID: &str = "optima-route-plotly-host";

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = Plotly)]
    fn purge(id: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn purge(_id: &str) {}

#[component]
pub fn PlotlyHost(#[prop(into)] label: String) -> impl IntoView {
    let host = NodeRef::<leptos::html::Div>::new();
    let theme = PlotlyTheme::optima();
    on_cleanup(move || {
        purge(HOST_ID);
        if let Some(element) = host.get_untracked() {
            element.set_inner_html("");
        }
    });
    view! {
        <div id=HOST_ID node_ref=host class="grid min-h-72 place-items-center rounded border border-dashed border-chart-grid bg-canvas"
            role="img" aria-label=label data-paper-background=theme.paper_background data-plot-background=theme.plot_background>
            <div class="max-w-sm px-5 text-center">
                <p class="text-xs font-semibold uppercase tracking-[0.16em] text-interactive-text">"Plotly lifecycle ready"</p>
                <p class="mt-2 text-xs leading-5 text-text-muted-readable">"The local WASM adapter is route-scoped. No financial trace is rendered in this milestone."</p>
            </div>
        </div>
    }
}
