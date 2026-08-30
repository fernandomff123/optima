use crate::{
    application::asset_simulation::SimulationLeg,
    driving_adapters::ui::simulation_draft::{
        DraftLeg, base_draft_legs, read_draft_legs, write_draft_legs,
    },
};
use leptos::prelude::*;

#[component]
pub fn SimulationPosition(
    symbol: String,
    strategy_name: String,
    legs: Vec<SimulationLeg>,
) -> impl IntoView {
    let base = base_draft_legs(&legs);
    let stored = read_draft_legs();
    let initial = if stored.is_empty() {
        base.clone()
    } else {
        stored
            .into_iter()
            .map(|stored_leg| {
                base.iter()
                    .find(|base_leg| base_leg.key == stored_leg.key)
                    .cloned()
                    .unwrap_or(stored_leg)
            })
            .collect()
    };
    write_draft_legs(&initial);
    let rows = RwSignal::new(initial);
    let (editing, set_editing) = signal(false);
    let (adding, set_adding) = signal(false);
    let options_path = format!("/assets/{symbol}/options");
    let chart_path = format!("/assets/{symbol}/chart");
    view! {
        <section class="relative flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Simulation position draft">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Position"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Browser draft"</span></div>
            <div class="p-3">
                <label class="sr-only" for="simulation-strategy">"Strategy"</label>
                <select id="simulation-strategy" class="min-h-9 w-full rounded border border-border bg-canvas px-3 text-xs text-text-primary" disabled>
                    <option>{strategy_name}</option>
                </select>
            </div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-x-auto">
                <table class="w-full min-w-[18rem] text-xs">
                    <thead class="border-y border-border bg-canvas text-text-secondary"><tr><th class="px-2 py-2 text-left font-medium">"Leg"</th><th class="px-2 py-2 text-left font-medium">"Type"</th><th class="px-2 py-2 text-right font-medium">"Strike"</th><th class="px-2 py-2 text-left font-medium">"Exp"</th><th class="px-2 py-2 text-right font-medium">"Qty"</th><th class="px-2 py-2 text-right font-medium">"Price"</th></tr></thead>
                    <tbody class="divide-y divide-border numeric">
                        {move || rows.get().into_iter().map(|leg| {
                            let quantity = leg.quantity;
                            let positive = quantity > 0;
                            let instrument = leg.instrument;
                            let strike = leg.strike;
                            let expiration = leg.expiration;
                            let price = leg.price;
                            let decrease_key = leg.key.clone();
                            let increase_key = leg.key.clone();
                            let remove_key = leg.key.clone();
                            view! {
                                <tr class="hover:bg-state-hover">
                                    <td class=if positive { "px-2 py-3 font-semibold text-finance-positive" } else { "px-2 py-3 font-semibold text-negative-text" }>{if positive { "+1" } else { "-1" }}</td>
                                    <td class=if positive { "px-2 py-3 font-semibold text-finance-positive" } else { "px-2 py-3 font-semibold text-negative-text" }>{instrument}</td>
                                    <td class="px-2 py-3 text-right text-text-primary">{strike}</td>
                                    <td class="px-2 py-3 text-text-primary">{expiration}</td>
                                    <td class="px-2 py-3 text-right text-text-primary">
                                        <span class=move || if editing.get() { "inline-flex items-center gap-1" } else { "hidden" }><button type="button" class="h-6 w-6 rounded border border-border text-text-secondary hover:bg-state-hover" aria-label="Decrease quantity" on:click=move |_| adjust_quantity(rows, &decrease_key, -1)>"−"</button><span class="min-w-5 text-center">{quantity.unsigned_abs()}</span><button type="button" class="h-6 w-6 rounded border border-border text-text-secondary hover:bg-state-hover" aria-label="Increase quantity" on:click=move |_| adjust_quantity(rows, &increase_key, 1)>"+"</button></span>
                                        <span class=move || if editing.get() { "hidden" } else { "inline" }>{quantity.unsigned_abs()}</span>
                                    </td>
                                    <td class="px-2 py-3 text-right text-text-primary"><button type="button" class=move || if editing.get() { "text-negative-text hover:underline" } else { "hidden" } on:click=move |_| remove_leg(rows, &remove_key)>"Remove"</button><span class=move || if editing.get() { "hidden" } else { "inline" }>{price}</span></td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
            <footer class="flex items-center justify-between border-t border-border px-3 py-3 text-xs">
                <button type="button" class="font-semibold text-interactive-text hover:underline" aria-expanded=move || adding.get() on:click=move |_| set_adding.update(|value| *value = !*value)>"＋ Add leg"</button>
                <button type="button" class=move || if editing.get() { "font-semibold text-finance-positive" } else { "font-semibold text-interactive-text hover:underline" } aria-pressed=move || editing.get() on:click=move |_| set_editing.update(|value| *value = !*value)>{move || if editing.get() { "Done" } else { "Edit Position" }}</button>
            </footer>
            {move || adding.get().then(|| view! {
                <div class="absolute bottom-12 left-3 z-20 w-56 rounded border border-border bg-surface p-2 shadow-2xl" role="dialog" aria-label="Choose leg source">
                    <p class="px-2 pb-2 text-[0.6875rem] text-text-secondary">"Choose the instrument, then use Add to Simulation."</p>
                    <a class="block rounded px-3 py-2 text-xs font-semibold text-interactive-text hover:bg-state-hover" href=options_path.clone()>"Option contract"</a>
                    <a class="block rounded px-3 py-2 text-xs font-semibold text-interactive-text hover:bg-state-hover" href=chart_path.clone()>"Underlying shares"</a>
                    <button type="button" class="mt-1 min-h-8 w-full rounded border border-border text-xs text-text-secondary hover:bg-state-hover" on:click=move |_| set_adding.set(false)>"Cancel"</button>
                </div>
            })}
        </section>
    }
}

fn adjust_quantity(rows: RwSignal<Vec<DraftLeg>>, key: &str, magnitude_delta: i32) {
    rows.update(|legs| {
        if let Some(leg) = legs.iter_mut().find(|leg| leg.key == key) {
            let sign = if leg.quantity < 0 { -1 } else { 1 };
            let magnitude = leg.quantity.unsigned_abs() as i32;
            leg.quantity = sign * (magnitude + magnitude_delta).max(1);
        }
        write_draft_legs(legs);
    });
}

fn remove_leg(rows: RwSignal<Vec<DraftLeg>>, key: &str) {
    rows.update(|legs| {
        legs.retain(|leg| leg.key != key);
        write_draft_legs(legs);
    });
}
