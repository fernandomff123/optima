use super::FinancialValue;
use crate::application::asset_overview::{DisplayMetric, DisplayRange};
use leptos::prelude::*;

#[component]
pub fn KeyStatistics(
    metrics: Vec<DisplayMetric>,
    year_range: Option<DisplayRange>,
) -> impl IntoView {
    let split_at = year_range
        .as_ref()
        .map_or(metrics.len(), |range| range.insert_after.min(metrics.len()));
    let leading = metrics[..split_at].to_vec();
    let trailing = metrics[split_at..].to_vec();
    view! {
        <dl class="flex h-full flex-col">
            <MetricRows metrics=leading />
            {year_range.map(|range| {
                let marker = format!("left: {}%", range.position.clamp(0.0, 1.0) * 100.0);
                view! {
                    <div class="flex min-h-[3.75rem] max-h-[4.5rem] flex-[1.5] flex-col justify-center border-b border-border px-3 py-2.5 last:border-b-0">
                        <div class="mb-2 flex items-center justify-between gap-4 text-sm">
                            <dt class="text-text-secondary">{range.label}</dt>
                            <dd class="numeric text-text-primary">{range.minimum} " – " {range.maximum}</dd>
                        </div>
                        <div class="relative h-1 bg-text-muted-source" role="meter" aria-label=range.accessible_value aria-valuemin="0" aria-valuemax="100" aria-valuenow=(range.position * 100.0).round()>
                            <span class="absolute -top-1.5 h-0 w-0 -translate-x-1/2 border-x-[5px] border-b-[8px] border-x-transparent border-b-interactive-text" style=marker></span>
                        </div>
                    </div>
                }
            })}
            <MetricRows metrics=trailing />
        </dl>
    }
}

#[component]
fn MetricRows(metrics: Vec<DisplayMetric>) -> impl IntoView {
    metrics.into_iter().map(|metric| {
        let label = metric.label;
        view! {
        <div class="fact-row min-h-9 max-h-10 flex-1">
            <dt class="text-text-secondary">{label.clone()}</dt>
            <dd class="ml-auto shrink-0 text-right"><FinancialValue value=metric.value unit=metric.unit tone=metric.tone label /></dd>
        </div>
    }}).collect_view()
}
