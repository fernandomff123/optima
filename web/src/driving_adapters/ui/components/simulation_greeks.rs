use crate::application::asset_simulation::Greek;
use leptos::prelude::*;

#[component]
pub fn SimulationGreeks(greeks: Vec<Greek>) -> impl IntoView {
    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Mock scenario Greeks">
            <div class="panel-header"><div><h2 class="text-sm font-semibold">"Greeks"</h2><p class="text-[0.625rem] uppercase tracking-wider text-level-special">"Current mock scenario"</p></div></div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-auto">
                <table class="w-full text-xs numeric">
                    <thead class="border-b border-border bg-canvas text-text-secondary"><tr><th class="px-4 py-2 text-left font-medium">"Greek"</th><th class="px-4 py-2 text-right font-medium">"Value"</th><th class="px-4 py-2 text-right font-medium">"Per 1% / $1 / 1 day"</th></tr></thead>
                    <tbody class="divide-y divide-border">{greeks.into_iter().map(|greek| view! { <tr class="hover:bg-state-hover"><th class="px-4 py-2.5 text-left font-medium text-text-primary">{greek.name}</th><td class="px-4 py-2.5 text-right text-text-primary">{greek.value}</td><td class="px-4 py-2.5 text-right text-text-secondary">{greek.sensitivity}</td></tr> }).collect_view()}</tbody>
                </table>
            </div>
        </section>
    }
}
