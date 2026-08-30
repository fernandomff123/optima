use crate::{
    application::asset_options::ContractDetail,
    driving_adapters::ui::simulation_draft::{
        contains_draft_leg, option_draft_leg, upsert_draft_leg,
    },
};
use leptos::prelude::*;

#[component]
pub fn OptionsContractPanel(contract: ContractDetail, symbol: String) -> impl IntoView {
    let draft_leg = option_draft_leg(&contract);
    let draft_key = draft_leg.key.clone();
    let (added_to_simulation, set_added_to_simulation) = signal(contains_draft_leg(&draft_key));
    let simulation_path = format!("/assets/{symbol}/simulation");
    view! {
        <aside class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Selected mock contract">
            <div class="dense-scrollbar min-h-0 flex-1 overflow-y-auto">
                <header class="border-b border-border px-4 py-4"><h2 class="text-sm font-bold text-text-primary">{contract.title}</h2><p class="mt-2 text-xs text-text-secondary">"Apple Inc."</p><div class="mt-5 flex items-baseline gap-3 numeric"><span class="text-3xl font-semibold text-text-primary">{contract.price}</span><span class="text-sm font-semibold text-finance-positive">{contract.change}</span></div></header>
                <div class="m-4 grid grid-cols-2 rounded border border-border bg-canvas text-xs numeric">
                    <div class="border-r border-border p-3"><p class="text-text-secondary">"Bid"</p><p class="mt-1 text-lg font-semibold text-interactive-text">{contract.bid}</p><p class="mt-1 text-text-secondary">"Size: " {contract.bid_size}</p></div>
                    <div class="p-3 text-right"><p class="text-text-secondary">"Ask"</p><p class="mt-1 text-lg font-semibold text-negative-text">{contract.ask}</p><p class="mt-1 text-text-secondary">"Size: " {contract.ask_size}</p></div>
                </div>
                <dl class="px-4 pb-4 text-xs numeric">{contract.metrics.into_iter().map(|(label, value)| view! { <div class="flex min-h-7 items-center justify-between gap-4"><dt class="text-text-secondary">{label}</dt><dd class="text-text-primary">{value}</dd></div> }).collect_view()}</dl>
                <div class="mx-4 border-t border-border"></div>
                <section class="px-4 py-4"><h3 class="mb-3 text-sm font-semibold text-text-primary">"Contract Details"</h3><dl class="text-xs numeric">{contract.facts.into_iter().map(|(label, value)| view! { <div class="flex min-h-7 items-center justify-between gap-4"><dt class="text-text-secondary">{label}</dt><dd class="text-right text-text-primary">{value}</dd></div> }).collect_view()}</dl></section>
            </div>
            <footer class="shrink-0 border-t border-border p-4">
                <button
                    type="button"
                    class=move || if added_to_simulation.get() { "min-h-10 w-full rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" } else { "min-h-10 w-full rounded border border-interactive-source bg-interactive-source px-4 text-xs font-semibold text-white hover:brightness-110" }
                    aria-pressed=move || added_to_simulation.get()
                    disabled=move || added_to_simulation.get()
                    on:click=move |_| {
                        if upsert_draft_leg(draft_leg.clone()) {
                            set_added_to_simulation.set(true);
                        }
                    }
                >
                    {move || if added_to_simulation.get() { "Added to Simulation" } else { "Add to Simulation" }}
                </button>
                <p class="sr-only" aria-live="polite">{move || added_to_simulation.get().then_some("Selected option added to the simulation")}</p>
                <a class=move || if added_to_simulation.get() { "mt-2 block min-h-9 rounded border border-border px-3 py-2 text-center text-xs font-semibold text-interactive-text hover:bg-state-hover" } else { "hidden" } href=simulation_path>"Open Simulation"</a>
            </footer>
        </aside>
    }
}
