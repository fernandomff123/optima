use crate::{
    application::asset_simulation::SimulationMetric, ports::asset_simulation::MetricSentiment,
};
use leptos::prelude::*;

#[component]
pub fn SimulationMetricStrip(metrics: Signal<Vec<SimulationMetric>>) -> impl IntoView {
    view! {
        <section class="grid border border-border bg-surface sm:grid-cols-5" aria-label="Mock simulation summary">
            {move || metrics.get().into_iter().map(|metric| {
                let tone = match metric.sentiment {
                    MetricSentiment::Positive => "text-finance-positive",
                    MetricSentiment::Negative => "text-negative-text",
                    MetricSentiment::Special => "text-level-special",
                    MetricSentiment::Neutral => "text-text-primary",
                };
                view! { <div class="flex min-h-[4.25rem] flex-col items-center justify-center border-b border-border px-3 text-center last:border-b-0 sm:border-b-0 sm:border-r sm:last:border-r-0"><p class="text-xs leading-none text-text-secondary">{metric.label}</p><p class=format!("mt-2.5 numeric text-lg font-semibold leading-none {tone}")>{metric.value}</p></div> }
            }).collect_view()}
        </section>
    }
}
