use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn OverviewMetric(metric: DisplayMetric) -> impl IntoView {
    view! {
        <div class="min-w-40 flex-1 border-r border-border px-4 py-3 last:border-r-0">
            <dt class="text-[0.6875rem] text-text-muted-readable">{metric.label}</dt>
            <dd class="numeric mt-1 text-xs font-medium text-text-primary">
                {metric.value.unwrap_or_else(|| "Unavailable".into())}
                {metric.unit.map(|unit| view! { <span class="ml-1 text-[0.625rem] text-text-muted-readable">{unit}</span> })}
            </dd>
        </div>
    }
}

#[component]
pub fn MetricStrip(metrics: Vec<DisplayMetric>) -> impl IntoView {
    view! { <dl class="dense-scrollbar flex overflow-x-auto border-b border-border bg-surface">{metrics.into_iter().map(|metric| view! { <OverviewMetric metric /> }).collect_view()}</dl> }
}
