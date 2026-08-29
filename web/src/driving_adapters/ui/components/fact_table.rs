use super::{FinancialValue, financial_value::value_tone_class};
use crate::application::asset_overview::DisplayMetric;
use leptos::prelude::*;

#[component]
pub fn FactTable(
    metrics: Vec<DisplayMetric>,
    #[prop(optional)] constrained: bool,
) -> impl IntoView {
    let table_class = if constrained {
        "w-full max-w-96 divide-y divide-border"
    } else {
        "divide-y divide-border"
    };
    view! {
        <dl class=table_class>
            {metrics.into_iter().map(|metric| {
                let label = metric.label;
                let tone_class = value_tone_class(metric.tone);
                let is_section = !metric.numeric && metric.value.is_none();
                let is_event = !metric.numeric && metric.suffix.is_some();
                let starts_group = metric.starts_group;
                let value = metric.value.unwrap_or_else(|| "Unavailable".into());
                let suffix = metric.suffix.unwrap_or_default();
                if is_section {
                    return view! {
                        <div class="flex h-[2.375rem] items-center border-b border-border px-3 text-sm last:border-b-0">
                            <dt class="whitespace-nowrap font-medium text-text-secondary">{label}</dt>
                        </div>
                    }.into_any();
                }
                let row_class = if constrained && starts_group {
                    "fact-row mt-2 grid h-[2.375rem] grid-cols-[minmax(0,1fr)_5.5rem] border-t border-border"
                } else if constrained {
                    "fact-row grid h-[2.375rem] grid-cols-[minmax(0,1fr)_5.5rem]"
                } else if is_event {
                    "fact-row h-14"
                } else {
                    "fact-row h-[2.375rem]"
                };
                view! {
                <div class=row_class>
                    <dt class="whitespace-nowrap text-text-secondary">{label.clone()}</dt>
                    <dd class=if constrained { "min-w-0 pl-3 text-right" } else { "ml-auto min-w-0 pl-3 text-right" }>
                        {if metric.numeric {
                            view! { <FinancialValue value=Some(value) suffix=Some(suffix) unit=metric.unit tone=metric.tone label /> }.into_any()
                        } else if is_event {
                            view! { <span class="block whitespace-nowrap"><span class=format!("block text-sm font-semibold {tone_class}")>{value}</span><span class="mt-0.5 block text-xs text-text-secondary">{suffix}</span></span> }.into_any()
                        } else {
                            view! { <span class=format!("whitespace-nowrap text-sm font-medium {tone_class}")>{value}</span> }.into_any()
                        }}
                    </dd>
                </div>
            }.into_any()}).collect_view()}
        </dl>
    }
}
