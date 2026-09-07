use crate::application::asset_options::{ContractDetail, OptionSelection};
use leptos::prelude::*;

#[component]
pub fn OptionsContractPanel(
    contract: Memo<ContractDetail>,
    company: String,
    symbol: String,
    selection: ReadSignal<Option<OptionSelection>>,
    feedback: ReadSignal<Option<String>>,
    draft_quantity: Memo<Option<i32>>,
) -> impl IntoView {
    let simulation_path = format!("/assets/{symbol}/simulation");
    view! {
        <aside class="flex h-full min-h-0 min-w-0 flex-col border border-border bg-surface" aria-label="Selected mock contract">
            <div class="dense-scrollbar min-h-0 flex-1 overflow-y-auto">
                <header class="border-b border-border px-2.5 py-2">
                    <div>
                        <h2 class="whitespace-nowrap text-xs font-bold text-text-primary">{move || contract.get().title}</h2>
                        <div class=move || if selection.get().is_some() { "mt-1 flex flex-wrap items-center justify-end gap-1 text-[0.625rem] font-bold tracking-wide" } else { "hidden" }>
                            <span class="rounded border border-border px-1.5 py-0.5 text-text-primary">{move || contract_kind(&contract.get())}</span>
                            <span class=move || match draft_quantity.get() {
                                Some(value) if value.is_positive() => "rounded border border-finance-positive bg-finance-positive/10 px-1.5 py-0.5 text-finance-positive",
                                Some(value) if value.is_negative() => "rounded border border-negative-text bg-negative-text/10 px-1.5 py-0.5 text-negative-text",
                                Some(_) => "rounded border border-border px-1.5 py-0.5 text-text-secondary",
                                None => "hidden",
                            }>{move || match draft_quantity.get() {
                                Some(value) if value.is_positive() => "LONG",
                                Some(value) if value.is_negative() => "SHORT",
                                Some(_) => "FLAT",
                                None => "",
                            }}</span>
                            <span class=move || match draft_quantity.get() {
                                Some(value) if value.is_positive() => "rounded border border-finance-positive bg-finance-positive/10 px-1.5 py-0.5 text-finance-positive",
                                Some(value) if value.is_negative() => "rounded border border-negative-text bg-negative-text/10 px-1.5 py-0.5 text-negative-text",
                                Some(_) => "rounded border border-border px-1.5 py-0.5 text-text-secondary",
                                None => "hidden",
                            }>{move || match draft_quantity.get() {
                                Some(0) => "Draft Qty 0".to_owned(),
                                Some(value) => format!("Draft Qty {value:+}"),
                                None => String::new(),
                            }}</span>
                        </div>
                    </div>
                    <p class="mt-1 text-[0.6875rem] text-text-secondary">{company.clone()}</p>
                    <div class="mt-3 flex items-baseline gap-2 numeric"><span class="text-3xl font-semibold text-text-primary">{move || contract.get().price}</span><span class="text-sm font-semibold text-finance-positive">{move || contract.get().change}</span></div>
                    <p class=move || if selection.get().is_some() { "mt-1 text-[0.6875rem] text-text-secondary" } else { "hidden" }>"Execution at selected " <span class="font-semibold text-text-primary">{move || contract.get().selected_quote}</span></p>
                </header>
                <div class="m-2 grid grid-cols-2 rounded border border-border bg-canvas text-xs numeric">
                    <div class=move || if selection.get().is_some() && contract.get().selected_quote == "Bid" { "border-r border-negative-text bg-negative-text/10 p-2.5" } else { "border-r border-border p-2.5" }><p class="text-text-secondary">"Bid · Sell"</p><p class="mt-1 text-lg font-semibold text-negative-text">{move || contract.get().bid}</p><p class="mt-1 text-text-secondary">"Size: " {move || contract.get().bid_size}</p></div>
                    <div class=move || if selection.get().is_some() && contract.get().selected_quote == "Ask" { "border border-interactive-source bg-state-selected p-2.5 text-right" } else { "p-2.5 text-right" }><p class="text-text-secondary">"Ask · Buy"</p><p class="mt-1 text-lg font-semibold text-interactive-text">{move || contract.get().ask}</p><p class="mt-1 text-text-secondary">"Size: " {move || contract.get().ask_size}</p></div>
                </div>
                <dl class="px-2.5 pb-2 text-xs numeric">{move || contract.get().metrics.into_iter().map(|(label, value)| view! { <div class="flex min-h-5 items-center justify-between gap-4"><dt class="text-text-secondary">{label}</dt><dd class="text-text-primary">{value}</dd></div> }).collect_view()}</dl>
                <details class="mx-2.5 border-t border-border">
                    <summary class="cursor-pointer py-2 text-xs font-semibold text-text-primary hover:text-interactive-text">"Contract Details"</summary>
                    <dl class="pb-2 text-[0.6875rem] numeric">{move || contract.get().facts.into_iter().map(|(label, value)| view! { <div class="flex min-h-5 items-center justify-between gap-4"><dt class="text-text-secondary">{label}</dt><dd class="text-right text-text-primary">{value}</dd></div> }).collect_view()}</dl>
                </details>
            </div>
            <footer class="shrink-0 border-t border-border p-2">
                <p class="min-h-7 rounded border border-border bg-canvas px-2 py-1.5 text-center text-[0.6875rem] text-text-secondary" aria-live="polite">{move || feedback.get().unwrap_or_else(|| "Select Bid to sell or Ask to buy".into())}</p>
                <a class="mt-1.5 block min-h-8 rounded border border-interactive-source px-2 py-1.5 text-center text-xs font-semibold text-interactive-text hover:bg-state-hover" href=simulation_path>"Open Simulation"</a>
            </footer>
        </aside>
    }
}

fn contract_kind(contract: &ContractDetail) -> String {
    contract
        .facts
        .iter()
        .find(|(label, _)| label.eq_ignore_ascii_case("Type"))
        .map(|(_, value)| value.to_uppercase())
        .unwrap_or_else(|| "OPTION".to_owned())
}
