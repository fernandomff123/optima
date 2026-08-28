use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn FactTable(metrics: Vec<DisplayMetric>) -> impl IntoView {
    view! {
        <dl class="divide-y divide-border">
            {metrics.into_iter().map(|metric| view! {
                <div class="fact-row">
                    <dt class="text-text-secondary">{metric.label}</dt>
                    <dd class="numeric text-right font-medium text-text-primary">
                        {metric.value.unwrap_or_else(|| "Unavailable".into())}
                        {metric.unit.map(|unit| view! { <span class="ml-1 text-xs font-normal text-text-muted-readable">{unit}</span> })}
                    </dd>
                </div>
            }).collect_view()}
        </dl>
    }
}
