use super::{FinancialValue, financial_value::value_tone_class};
use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn FactTable(metrics: Vec<DisplayMetric>) -> impl IntoView {
    view! {
        <dl class="divide-y divide-border">
            {metrics.into_iter().map(|metric| {
                let label = metric.label;
                let tone_class = value_tone_class(metric.tone);
                view! {
                <div class="fact-row">
                    <dt class="text-text-secondary">{label.clone()}</dt>
                    <dd class="ml-auto min-w-0 pl-3 text-right">
                        {if metric.numeric {
                            view! { <FinancialValue value=metric.value suffix=metric.suffix unit=metric.unit tone=metric.tone label /> }.into_any()
                        } else {
                            view! { <span class=format!("text-sm font-medium sm:whitespace-nowrap {tone_class}")>{metric.value.unwrap_or_else(|| "Unavailable".into())}</span> }.into_any()
                        }}
                    </dd>
                </div>
            }}).collect_view()}
        </dl>
    }
}
