use crate::application::asset_volatility::VolatilityMetric;
use leptos::prelude::*;

#[component]
pub fn VolatilitySnapshot(metrics: Vec<VolatilityMetric>, as_of: String) -> impl IntoView {
    view! {
        <aside class="flex min-h-0 flex-col border border-border bg-surface" aria-label="Volatility snapshot">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Volatility Snapshot"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Mock"</span></div>
            <div class="min-h-0 flex-1 overflow-auto">
                <table class="w-full text-sm"><thead><tr class="border-b border-border text-xs text-text-secondary"><th class="px-4 py-3 text-left font-medium">"Metric"</th><th class="px-4 py-3 text-right font-medium">"Value"</th></tr></thead><tbody>{metrics.into_iter().map(|metric| { let special = metric.label == "25Δ Skew"; view! { <tr class="border-b border-border last:border-b-0"><th class="px-4 py-3 text-left font-medium text-text-primary">{metric.label}</th><td class="px-4 py-3 text-right numeric" class:text-level-special=special>{metric.value}</td></tr> } }).collect_view()}</tbody></table>
            </div>
            <p class="border-t border-border px-4 py-3 text-[0.6875rem] text-text-secondary">{as_of}</p>
        </aside>
    }
}
