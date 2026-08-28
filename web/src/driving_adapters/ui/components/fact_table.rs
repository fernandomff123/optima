use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn FactTable(metrics: Vec<DisplayMetric>) -> impl IntoView {
    view! {
        <dl class="divide-y divide-border">
            {metrics.into_iter().map(|metric| view! {
                <div class="flex min-h-8 items-center justify-between gap-4 py-2 text-xs">
                    <dt class="text-text-secondary">{metric.label}</dt>
                    <dd class="numeric text-right text-text-primary">
                        {metric.value.unwrap_or_else(|| "Unavailable".into())}
                        {metric.unit.map(|unit| view! { <span class="ml-1 text-[0.625rem] text-text-muted-readable">{unit}</span> })}
                    </dd>
                </div>
            }).collect_view()}
        </dl>
    }
}
