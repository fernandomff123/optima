use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn OverviewMetric(metric: DisplayMetric) -> impl IntoView {
    view! {
        <div class="min-w-40 flex-1 border-r border-border px-4 py-2.5 last:border-r-0 sm:px-5">
            <dt class="metric-label">{metric.label}</dt>
            <dd class="metric-value mt-0.5">
                {metric.value.unwrap_or_else(|| "Unavailable".into())}
                {metric.unit.map(|unit| view! { <span class="ml-1 text-xs text-text-muted-readable">{unit}</span> })}
            </dd>
        </div>
    }
}

#[component]
pub fn MetricStrip(metrics: Vec<DisplayMetric>) -> impl IntoView {
    view! { <dl class="dense-scrollbar flex overflow-x-auto border-b border-border bg-surface">{metrics.into_iter().map(|metric| view! { <OverviewMetric metric /> }).collect_view()}</dl> }
}
