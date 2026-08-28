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
        <dl>
            <MetricRows metrics=leading />
            {year_range.map(|range| {
                let marker = format!("left: {}%", range.position.clamp(0.0, 1.0) * 100.0);
                view! {
                    <div class="border-b border-border px-3 py-2.5 last:border-b-0">
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
    metrics.into_iter().map(|metric| view! {
        <div class="fact-row">
            <dt class="text-text-secondary">{metric.label}</dt>
            <dd class="numeric text-right font-medium text-text-primary">
                {metric.value.unwrap_or_else(|| "Unavailable".into())}
                {metric.unit.map(|unit| view! { <span class="ml-1 text-xs font-normal text-text-muted-readable">{unit}</span> })}
            </dd>
        </div>
    }).collect_view()
}
