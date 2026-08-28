use super::FinancialValue;
use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn FactTable(metrics: Vec<DisplayMetric>) -> impl IntoView {
    view! {
        <dl class="divide-y divide-border">
            {metrics.into_iter().map(|metric| {
                let label = metric.label;
                view! {
                <div class="fact-row">
                    <dt class="text-text-secondary">{label.clone()}</dt>
                    <dd class="ml-auto shrink-0 text-right"><FinancialValue value=metric.value unit=metric.unit tone=metric.tone label /></dd>
                </div>
            }}).collect_view()}
        </dl>
    }
}
