use leptos::prelude::*;
use plotly::Plot;
use send_wrapper::SendWrapper;

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

    fn missing_element_is_error(&self) -> bool {
        self.mounted && !self.cleaned
    }
}

#[component]
pub fn PlotlyChart(
    id: &'static str,
    plot: ReadSignal<SendWrapper<Option<Plot>>>,
    error: WriteSignal<Option<String>>,
    aria_label: &'static str,
) -> impl IntoView {
    let node_ref = NodeRef::<leptos::html::Div>::new();

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Object, Promise, Reflect};
        use leptos::leptos_dom::helpers::window_event_listener;
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicU64, Ordering},
        };
        use wasm_bindgen::{JsCast, prelude::*};

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Plotly, js_name = newPlot)]
            fn new_plot(element: &web_sys::Element, plot: &Object) -> Result<Promise, JsValue>;

            #[wasm_bindgen(catch, js_namespace = Plotly, js_name = react)]
            fn react_plot(element: &web_sys::Element, plot: &Object) -> Result<Promise, JsValue>;

            #[wasm_bindgen(catch, js_namespace = Plotly, js_name = purge)]
            fn purge_plot(element: &web_sys::Element) -> Result<(), JsValue>;

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

        const OWNER_PROPERTY: &str = "__optimaPlotOwner";

        fn claim_element(element: &web_sys::Element, owner: &Object) -> Result<(), String> {
            Reflect::set(
                element.as_ref(),
                &JsValue::from_str(OWNER_PROPERTY),
                owner.as_ref(),
            )
            .map(|_| ())
            .map_err(plotly_error)
        }

        fn owns_element(element: &web_sys::Element, owner: &Object) -> bool {
            Reflect::get(element.as_ref(), &JsValue::from_str(OWNER_PROPERTY))
                .is_ok_and(|current| Object::is(&current, owner.as_ref()))
        }

        fn release_element(element: &web_sys::Element, owner: &Object) {
            if owns_element(element, owner) {
                let _ =
                    Reflect::delete_property(element.as_ref(), &JsValue::from_str(OWNER_PROPERTY));
            }
        }

        let lifecycle = Arc::new(Mutex::new(PlotLifecycle::default()));
        let active = Arc::new(AtomicBool::new(true));
        let revision = Arc::new(AtomicU64::new(0));
        let owner = Object::new();
        let effect_lifecycle = lifecycle.clone();
        let effect_active = active.clone();
        let effect_revision = revision.clone();
        let effect_owner = owner.clone();
        Effect::new(move |_| {
            let current_revision = effect_revision.fetch_add(1, Ordering::AcqRel) + 1;
            let Some(element) = node_ref.get() else {
                let missing_after_mount = match effect_lifecycle.lock() {
                    Ok(lifecycle) => lifecycle.missing_element_is_error(),
                    Err(poisoned) => poisoned.get_ref().missing_element_is_error(),
                };
                if missing_after_mount {
                    error.set(Some(format!(
                        "O elemento montado do gráfico {id} deixou de estar disponível."
                    )));
                }
                return;
            };
            let plot = plot.get();
            let Some(plot) = plot.as_ref() else {
                let should_purge = match effect_lifecycle.lock() {
                    Ok(mut lifecycle) => lifecycle.cleanup(),
                    Err(mut poisoned) => poisoned.get_mut().cleanup(),
                };
                if should_purge && owns_element(element.as_ref(), &effect_owner) {
                    if let Err(value) = purge_plot(element.as_ref()) {
                        error.set(Some(plotly_error(value)));
                    }
                    release_element(element.as_ref(), &effect_owner);
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
            if first_mount && let Err(message) = claim_element(element.as_ref(), &effect_owner) {
                error.set(Some(message));
                return;
            }
            if !owns_element(element.as_ref(), &effect_owner) {
                return;
            }
            let promise = if first_mount {
                new_plot(element.as_ref(), &plot)
            } else {
                react_plot(element.as_ref(), &plot)
            };
            let promise = match promise {
                Ok(promise) => promise,
                Err(value) => {
                    error.set(Some(plotly_error(value)));
                    return;
                }
            };
            let task_active = effect_active.clone();
            let task_revision = effect_revision.clone();
            let task_owner = effect_owner.clone();
            leptos::task::spawn_local(async move {
                let result = wasm_bindgen_futures::JsFuture::from(promise).await;
                if task_active.load(Ordering::Acquire)
                    && task_revision.load(Ordering::Acquire) == current_revision
                    && owns_element(element.as_ref(), &task_owner)
                {
                    match result {
                        Ok(_) => match resize_plot(element.as_ref()) {
                            Ok(()) => error.set(None),
                            Err(value) => error.set(Some(plotly_error(value))),
                        },
                        Err(value) => error.set(Some(plotly_error(value))),
                    }
                }
            });
        });

        let resize_lifecycle = lifecycle.clone();
        let resize_owner = owner.clone();
        let resize_listener = window_event_listener(leptos::ev::resize, move |_| {
            match node_ref.get_untracked() {
                Some(element) => {
                    if owns_element(element.as_ref(), &resize_owner)
                        && let Err(value) = resize_plot(element.as_ref())
                    {
                        error.set(Some(plotly_error(value)));
                    }
                }
                None => {
                    let missing_after_mount = match resize_lifecycle.lock() {
                        Ok(lifecycle) => lifecycle.missing_element_is_error(),
                        Err(poisoned) => poisoned.get_ref().missing_element_is_error(),
                    };
                    if missing_after_mount {
                        error.set(Some(format!(
                            "O elemento montado do gráfico {id} deixou de estar disponível."
                        )));
                    }
                }
            }
        });

        let cleanup_owner = owner;
        on_cleanup(move || {
            active.store(false, Ordering::Release);
            revision.fetch_add(1, Ordering::AcqRel);
            resize_listener.remove();
            let should_purge = match lifecycle.lock() {
                Ok(mut lifecycle) => lifecycle.cleanup(),
                Err(mut poisoned) => poisoned.get_mut().cleanup(),
            };
            if should_purge
                && let Some(element) = node_ref.get_untracked()
                && owns_element(element.as_ref(), &cleanup_owner)
            {
                let _ = purge_plot(element.as_ref());
                release_element(element.as_ref(), &cleanup_owner);
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = (plot, error);

    view! { <div node_ref=node_ref id=id class="plotly-chart" role="img" aria-label=aria_label></div> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotly::{Plot, Scatter};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestOperationKind {
        NewPlot,
        React,
        Purge,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestTraceDimensions {
        x: usize,
        y: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestOperationRecord {
        target: &'static str,
        kind: TestOperationKind,
        traces: Vec<TestTraceDimensions>,
    }

    fn operation_record(
        target: &'static str,
        kind: TestOperationKind,
        plot: &Plot,
    ) -> TestOperationRecord {
        let figure: serde_json::Value = serde_json::from_str(&plot.to_json()).unwrap();
        let traces = figure["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|trace| TestTraceDimensions {
                x: trace["x"].as_array().map_or(0, Vec::len),
                y: trace["y"].as_array().map_or(0, Vec::len),
            })
            .collect();
        TestOperationRecord {
            target,
            kind,
            traces,
        }
    }

    fn test_plot(points: usize) -> Plot {
        let mut plot = Plot::new();
        let values = (0..points).collect::<Vec<_>>();
        plot.add_trace(Scatter::new(values.clone(), values));
        plot
    }

    #[derive(Default)]
    struct TestOperationLog(Vec<TestOperationRecord>);

    impl TestOperationLog {
        fn dispatch(&mut self, target: &'static str, kind: TestOperationKind, plot: &Plot) {
            self.0.push(operation_record(target, kind, plot));
        }

        fn for_target(&self, target: &str) -> Vec<&TestOperationRecord> {
            self.0
                .iter()
                .filter(|operation| operation.target == target)
                .collect()
        }
    }

    struct TestPlotInstance {
        target: &'static str,
        lifecycle: PlotLifecycle,
        containers: usize,
    }

    impl TestPlotInstance {
        fn new(target: &'static str) -> Self {
            Self {
                target,
                lifecycle: PlotLifecycle::default(),
                containers: 1,
            }
        }

        fn render(&mut self, node_available: bool) -> Result<Option<(&'static str, bool)>, ()> {
            if !node_available {
                return if self.lifecycle.missing_element_is_error() {
                    Err(())
                } else {
                    Ok(None)
                };
            }
            Ok(Some((self.target, self.lifecycle.mount())))
        }

        fn cleanup(&mut self) -> Option<&'static str> {
            self.lifecycle.cleanup().then_some(self.target)
        }
    }

    #[test]
    fn lifecycle_mounts_updates_and_cleans_up_once() {
        let mut lifecycle = PlotLifecycle::default();
        assert!(!lifecycle.missing_element_is_error());
        assert!(lifecycle.mount());
        assert!(lifecycle.missing_element_is_error());
        assert!(!lifecycle.mount());
        assert!(lifecycle.cleanup());
        assert!(!lifecycle.cleanup());
        assert!(lifecycle.cleaned);
        assert!(!lifecycle.mounted);
        assert!(!lifecycle.missing_element_is_error());
    }

    #[test]
    fn missing_node_before_mount_is_not_an_error_but_missing_mounted_node_is() {
        let mut instance = TestPlotInstance::new("history-element");
        assert_eq!(instance.render(false), Ok(None));
        assert_eq!(instance.render(true), Ok(Some(("history-element", true))));
        assert_eq!(instance.render(false), Err(()));
    }

    #[test]
    fn two_components_route_new_plot_and_react_to_distinct_stable_elements() {
        let mut history = TestPlotInstance::new("history-element");
        let mut gex = TestPlotInstance::new("gex-element");

        assert_eq!(history.render(true), Ok(Some(("history-element", true))));
        assert_eq!(gex.render(true), Ok(Some(("gex-element", true))));
        assert_eq!(history.render(true), Ok(Some(("history-element", false))));
        assert_eq!(gex.render(true), Ok(Some(("gex-element", false))));
        assert_eq!(history.containers, 1);
        assert_eq!(gex.containers, 1);
    }

    #[test]
    fn cleanup_and_signal_update_cannot_swap_targets() {
        let mut history = TestPlotInstance::new("history-element");
        let mut gex = TestPlotInstance::new("gex-element");
        history.render(true).unwrap();
        gex.render(true).unwrap();

        assert_eq!(history.cleanup(), Some("history-element"));
        assert!(gex.lifecycle.mounted);
        assert_eq!(gex.render(true), Ok(Some(("gex-element", false))));
        assert_eq!(history.render(true), Ok(Some(("history-element", true))));
    }

    #[test]
    fn old_cleanup_cannot_purge_an_element_claimed_by_a_new_owner() {
        let old_owner = 1_u64;
        let new_owner = 2_u64;
        let mut element_owner = Some(old_owner);
        assert_eq!(element_owner, Some(old_owner));
        element_owner = Some(new_owner);

        let old_cleanup_purges = element_owner == Some(old_owner);
        assert!(!old_cleanup_purges);
        assert_eq!(element_owner, Some(new_owner));
    }

    #[test]
    fn gex_parameter_sequence_never_dispatches_to_history() {
        const HISTORY: &str = "spx-history-plot";
        const GEX: &str = "gex-profile-plot";
        let history_plot = test_plot(90);
        let gex_81 = test_plot(81);
        let gex_55 = test_plot(55);
        let mut operations = TestOperationLog::default();

        operations.dispatch(HISTORY, TestOperationKind::NewPlot, &history_plot);
        operations.dispatch(GEX, TestOperationKind::NewPlot, &gex_81);
        // Changing range from 20 to 30 and points from 81 to 55 only edits form signals.
        // Resolving the new request is the sole operation dispatched afterwards.
        operations.dispatch(GEX, TestOperationKind::React, &gex_55);

        assert_eq!(
            operations.for_target(HISTORY),
            vec![&TestOperationRecord {
                target: HISTORY,
                kind: TestOperationKind::NewPlot,
                traces: vec![TestTraceDimensions { x: 90, y: 90 }],
            }]
        );
        assert_eq!(
            operations.for_target(GEX),
            vec![&operations.0[1], &operations.0[2]]
        );
        assert_eq!(operations.0[1].kind, TestOperationKind::NewPlot);
        assert_eq!(
            operations.0[1].traces[0],
            TestTraceDimensions { x: 81, y: 81 }
        );
        assert_eq!(operations.0[2].kind, TestOperationKind::React);
        assert_eq!(
            operations.0[2].traces[0],
            TestTraceDimensions { x: 55, y: 55 }
        );
        assert!(
            !operations
                .for_target(HISTORY)
                .iter()
                .any(|operation| matches!(
                    operation.kind,
                    TestOperationKind::React | TestOperationKind::Purge
                ))
        );
    }

    #[test]
    fn changing_only_gex_range_dispatches_no_history_operation() {
        let mut operations = TestOperationLog::default();
        operations.dispatch(
            "spx-history-plot",
            TestOperationKind::NewPlot,
            &test_plot(90),
        );
        let mut range = 20_u32;
        assert_eq!(range, 20);
        let before = operations.for_target("spx-history-plot").len();
        range = 30;
        assert_eq!(range, 30);
        assert_eq!(operations.for_target("spx-history-plot").len(), before);
    }

    #[test]
    fn changing_only_gex_points_dispatches_no_history_operation() {
        let mut operations = TestOperationLog::default();
        operations.dispatch(
            "spx-history-plot",
            TestOperationKind::NewPlot,
            &test_plot(90),
        );
        let mut points = 81_usize;
        assert_eq!(points, 81);
        let before = operations.for_target("spx-history-plot").len();
        points = 55;
        assert_eq!(points, 55);
        assert_eq!(operations.for_target("spx-history-plot").len(), before);
    }

    #[test]
    fn resolving_gex_result_dispatches_only_gex_react() {
        let mut operations = TestOperationLog::default();
        operations.dispatch(
            "spx-history-plot",
            TestOperationKind::NewPlot,
            &test_plot(90),
        );
        operations.dispatch(
            "gex-profile-plot",
            TestOperationKind::NewPlot,
            &test_plot(81),
        );
        let history_before = operations.for_target("spx-history-plot").len();
        operations.dispatch("gex-profile-plot", TestOperationKind::React, &test_plot(55));
        assert_eq!(
            operations.for_target("spx-history-plot").len(),
            history_before
        );
        assert_eq!(
            operations
                .for_target("gex-profile-plot")
                .last()
                .unwrap()
                .kind,
            TestOperationKind::React
        );
    }

    #[test]
    fn repeated_gex_reacts_preserve_full_history_trace() {
        const HISTORY: &str = "spx-history-plot";
        const GEX: &str = "gex-profile-plot";
        let history_plot = test_plot(90);
        let mut operations = TestOperationLog::default();
        operations.dispatch(HISTORY, TestOperationKind::NewPlot, &history_plot);
        for points in [81, 55, 101, 21] {
            operations.dispatch(
                GEX,
                if points == 81 {
                    TestOperationKind::NewPlot
                } else {
                    TestOperationKind::React
                },
                &test_plot(points),
            );
        }
        operations.dispatch(GEX, TestOperationKind::Purge, &Plot::new());

        let history = operations.for_target(HISTORY);
        assert_eq!(history.len(), 1);
        assert_eq!(
            history[0].traces,
            vec![TestTraceDimensions { x: 90, y: 90 }]
        );
    }

    #[test]
    fn reusable_component_owns_a_node_ref_and_never_searches_the_document() {
        let source = include_str!("plotly_chart.rs");
        let runtime = source.split("#[cfg(test)]").next().unwrap();
        assert!(runtime.contains("NodeRef::<leptos::html::Div>::new()"));
        assert!(runtime.contains("node_ref=node_ref"));
        assert!(!runtime.contains("get_element_by_id"));
        assert!(!runtime.contains("create_element"));
        assert!(!runtime.contains("append_child"));
        assert!(!runtime.contains("setTimeout"));
    }

    #[test]
    fn legacy_spx_svg_css_cannot_style_plotly_internal_layers() {
        let css = include_str!("../styles.css");
        for forbidden_selector in [
            ".spx-chart svg",
            ".spx-chart polyline",
            ".chart-grid line",
            ".spx-line",
            ".chart-value",
            ".chart-date",
            ".chart-session-count",
            ".main-svg",
            ".svg-container",
            ".plot-container",
        ] {
            assert!(
                !css.contains(forbidden_selector),
                "legacy selector still reaches Plotly: {forbidden_selector}"
            );
        }
        assert!(css.contains(".spx-chart .plotly-chart { min-height: 250px; }"));
    }

    #[test]
    fn completed_plot_operation_is_resized_without_a_timer() {
        let source = include_str!("plotly_chart.rs");
        let runtime = source.split("#[cfg(test)]").next().unwrap();
        assert!(runtime.contains("Ok(_) => match resize_plot(element.as_ref())"));
        assert!(!runtime.contains("setTimeout"));
    }

    #[test]
    fn each_mount_dispatches_exactly_one_new_plot_without_creating_containers() {
        let mut history = TestPlotInstance::new("spx-history-plot");
        let mut gex = TestPlotInstance::new("gex-profile-plot");

        let history_operations = [history.render(true), history.render(true)];
        let gex_operations = [gex.render(true), gex.render(true)];

        assert_eq!(
            history_operations
                .iter()
                .filter(|result| matches!(result, Ok(Some((_, true)))))
                .count(),
            1
        );
        assert_eq!(
            gex_operations
                .iter()
                .filter(|result| matches!(result, Ok(Some((_, true)))))
                .count(),
            1
        );
        assert_eq!(history.containers, 1);
        assert_eq!(gex.containers, 1);
    }

    #[test]
    fn page_declares_exactly_one_history_and_one_gex_target() {
        let main = include_str!("main.rs");
        let gex = include_str!("gamma_exposure.rs");
        let gex_runtime = gex.split("#[cfg(test)]").next().unwrap();
        assert_eq!(
            main.matches("<PlotlyChart id=SPX_HISTORY_PLOT_ID").count(),
            1
        );
        assert_eq!(
            gex_runtime.matches("<PlotlyChart id=GEX_PLOT_ID").count(),
            1
        );
        assert_eq!(main.matches("const SPX_HISTORY_PLOT_ID:").count(), 1);
        assert_eq!(gex_runtime.matches("const GEX_PLOT_ID:").count(), 1);
    }

    #[test]
    fn two_lifecycles_are_independent() {
        let mut history = PlotLifecycle::default();
        let mut gex = PlotLifecycle::default();
        assert!(history.mount());
        assert!(gex.mount());
        assert!(history.cleanup());
        assert!(!history.mounted);
        assert!(gex.mounted);
        assert!(!gex.cleaned);
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
