use leptos::prelude::*;
use plotly::Plot;
use send_wrapper::SendWrapper;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PLOT_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Default)]
struct PlotLifecycle {
    mounted: bool,
    cleaned: bool,
}

#[cfg(any(target_arch = "wasm32", test))]
impl PlotLifecycle {
    fn mount(&mut self) -> bool {
        let first_mount = !self.mounted;
        self.mounted = true;
        self.cleaned = false;
        first_mount
    }

    fn cleanup(&mut self) -> bool {
        let should_purge = self.mounted;
        self.cleaned = true;
        self.mounted = false;
        should_purge
    }
}

#[component]
pub fn PlotlyChart(
    plot: ReadSignal<SendWrapper<Option<Plot>>>,
    error: WriteSignal<Option<String>>,
    aria_label: &'static str,
) -> impl IntoView {
    let id = format!(
        "plotly-chart-{}",
        NEXT_PLOT_ID.fetch_add(1, Ordering::Relaxed)
    );

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Object;
        use leptos::leptos_dom::helpers::window_event_listener;
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicU64, Ordering},
        };
        use wasm_bindgen::{JsCast, prelude::*};

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Plotly, js_name = newPlot)]
            async fn new_plot(id: &str, plot: &Object) -> Result<JsValue, JsValue>;

            #[wasm_bindgen(catch, js_namespace = Plotly, js_name = react)]
            async fn react_plot(id: &str, plot: &Object) -> Result<JsValue, JsValue>;

            #[wasm_bindgen(catch, js_namespace = Plotly, js_name = purge)]
            fn purge_plot(id: &str) -> Result<(), JsValue>;

            #[wasm_bindgen(catch, js_namespace = ["Plotly", "Plots"], js_name = resize)]
            fn resize_plot(element: &web_sys::Element) -> Result<(), JsValue>;
        }

        fn plotly_error(value: JsValue) -> String {
            match value.as_string() {
                Some(message) => message,
                None => "Plotly.js não está disponível.".to_string(),
            }
        }

        fn plot_object(plot: &Plot) -> Result<Object, String> {
            let json = serde_json::to_string(plot)
                .map_err(|error| format!("Não foi possível serializar o gráfico: {error}"))?;
            let value = js_sys::JSON::parse(&json).map_err(plotly_error)?;
            value
                .dyn_into::<Object>()
                .map_err(|value| plotly_error(value.into()))
        }

        let lifecycle = Arc::new(Mutex::new(PlotLifecycle::default()));
        let active = Arc::new(AtomicBool::new(true));
        let revision = Arc::new(AtomicU64::new(0));
        let effect_id = id.clone();
        let effect_lifecycle = lifecycle.clone();
        let effect_active = active.clone();
        let effect_revision = revision.clone();
        Effect::new(move |_| {
            let current_revision = effect_revision.fetch_add(1, Ordering::AcqRel) + 1;
            let plot = plot.get();
            let Some(plot) = plot.as_ref() else {
                let should_purge = match effect_lifecycle.lock() {
                    Ok(mut lifecycle) => lifecycle.cleanup(),
                    Err(mut poisoned) => poisoned.get_mut().cleanup(),
                };
                if should_purge && let Err(value) = purge_plot(&effect_id) {
                    error.set(Some(plotly_error(value)));
                }
                return;
            };
            let plot = match plot_object(plot) {
                Ok(plot) => plot,
                Err(message) => {
                    error.set(Some(message));
                    return;
                }
            };
            let first_mount = match effect_lifecycle.lock() {
                Ok(mut lifecycle) => lifecycle.mount(),
                Err(mut poisoned) => poisoned.get_mut().mount(),
            };
            let effect_id = effect_id.clone();
            let task_active = effect_active.clone();
            let task_revision = effect_revision.clone();
            leptos::task::spawn_local(async move {
                let result = if first_mount {
                    new_plot(&effect_id, &plot).await
                } else {
                    react_plot(&effect_id, &plot).await
                };
                if task_active.load(Ordering::Acquire)
                    && task_revision.load(Ordering::Acquire) == current_revision
                {
                    match result {
                        Ok(_) => error.set(None),
                        Err(value) => error.set(Some(plotly_error(value))),
                    }
                }
            });
        });

        let resize_id = id.clone();
        let resize_listener = window_event_listener(leptos::ev::resize, move |_| {
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(&resize_id))
                && let Err(value) = resize_plot(&element)
            {
                error.set(Some(plotly_error(value)));
            }
        });

        let cleanup_id = id.clone();
        on_cleanup(move || {
            active.store(false, Ordering::Release);
            revision.fetch_add(1, Ordering::AcqRel);
            resize_listener.remove();
            let should_purge = match lifecycle.lock() {
                Ok(mut lifecycle) => lifecycle.cleanup(),
                Err(mut poisoned) => poisoned.get_mut().cleanup(),
            };
            if should_purge {
                let _ = purge_plot(&cleanup_id);
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = (plot, error);

    view! { <div id=id class="plotly-chart" role="img" aria-label=aria_label></div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_mounts_updates_and_cleans_up_once() {
        let mut lifecycle = PlotLifecycle::default();
        assert!(lifecycle.mount());
        assert!(!lifecycle.mount());
        assert!(lifecycle.cleanup());
        assert!(!lifecycle.cleanup());
        assert!(lifecycle.cleaned);
        assert!(!lifecycle.mounted);
    }

    #[test]
    fn base_html_loads_plotly_once() {
        let html = include_str!("../index.html");
        assert_eq!(
            html.matches("https://cdn.plot.ly/plotly-3.0.1.min.js")
                .count(),
            1
        );
    }
}
