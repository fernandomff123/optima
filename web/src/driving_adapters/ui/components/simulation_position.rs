use super::{ShellIcon, ShellIconKind};
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
    spot_price: String,
) -> impl IntoView {
    let base = base_draft_legs(&legs);
    let stored = read_draft_legs();
    let mut initial = if stored.is_empty() {
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
    for leg in &mut initial {
        if leg.instrument.eq_ignore_ascii_case("STOCK") && leg.price.eq_ignore_ascii_case("Market")
        {
            leg.price = spot_price.clone();
        }
    }
    write_draft_legs(&initial);
    let rows = RwSignal::new(initial);
    let options_path = format!("/assets/{symbol}/options");
    view! {
            <section class="relative flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Simulation position draft">
                <div class="panel-header"><h2 class="text-sm font-semibold">"Position"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Browser draft"</span></div>
                <div class="p-3">
                    <label class="sr-only" for="simulation-strategy">"Strategy"</label>
                    <select id="simulation-strategy" class="min-h-9 w-full rounded border border-border bg-canvas px-3 text-xs text-text-primary" disabled>
                        <option>{strategy_name}</option>
                    </select>
                </div>
                <div class="dense-scrollbar min-h-0 flex-1 overflow-auto">
                    <table class="w-full min-w-[25.5rem] table-fixed text-[0.6875rem]">
                        <thead class="border-y border-border bg-canvas text-text-secondary"><tr><th class="w-7 px-1 py-2"><span class="sr-only">"Actions"</span></th><th class="w-10 px-1.5 py-2 text-left font-medium">"Side"</th><th class="w-12 px-1.5 py-2 text-left font-medium">"Type"</th><th class="w-11 px-1.5 py-2 text-right font-medium">"Strike"</th><th class="w-14 px-1.5 py-2 text-left font-medium">"Exp"</th><th class="w-14 px-1.5 py-2 text-right font-medium">"Qty"</th><th class="w-12 px-1.5 py-2 text-right font-medium">"Price"</th><th class="w-[5.5rem] px-1.5 py-2 text-right font-medium">"Debit/Credit"</th></tr></thead>
                        <tbody class="divide-y divide-border numeric">
                            {move || rows.get().into_iter().map(|leg| {
                                let quantity = leg.quantity;
                                let positive = quantity > 0;
                                let instrument = leg.instrument;
                                let strike = leg.strike;
                                let expiration = leg.expiration;
                                let price = leg.price;
                            let cash_flow = format_cash_flow(quantity, &instrument, &price);
                            let cash_flow_class = if quantity < 0 {
                                "px-1.5 py-3 text-right font-semibold text-finance-positive"
                            } else {
                                "px-1.5 py-3 text-right font-semibold text-negative-text"
                            };
                                let increase_key = leg.key.clone();
                                let decrease_key = leg.key.clone();
                                let remove_key = leg.key.clone();
                                view! {
                                    <tr class="hover:bg-state-hover">
                                        <td class="px-1 py-3 text-center"><button type="button" class="inline-flex size-6 items-center justify-center rounded text-text-secondary hover:bg-negative-bg hover:text-negative-text" aria-label="Remove leg" on:click=move |_| remove_leg(rows, &remove_key)><ShellIcon kind=ShellIconKind::Trash class="size-3.5" /></button></td>
                                        <td class=if positive { "px-1.5 py-3 font-semibold text-interactive-text" } else { "px-1.5 py-3 font-semibold text-negative-text" }>{if positive { "BUY" } else { "SELL" }}</td>
                                        <td class="px-1.5 py-3 font-semibold text-text-primary">{instrument}</td>
                                        <td class="px-1.5 py-3 text-right text-text-primary">{strike}</td>
                                        <td class="px-1.5 py-3 leading-tight text-text-primary">{expiration}</td>
                                        <td class="px-1.5 py-2 text-right text-text-primary">
                                        <span class="inline-flex items-center justify-end gap-1">
                                            <span class="min-w-4 text-right">{quantity.unsigned_abs()}</span>
                                            <span class="inline-flex flex-col gap-px">
                                                <button type="button" class="flex h-3.5 w-4 items-center justify-center rounded-sm border border-border text-[0.625rem] leading-none text-text-secondary hover:bg-state-hover hover:text-text-primary" aria-label="Increase quantity" on:click=move |_| adjust_quantity(rows, &increase_key, 1)>"+"</button>
                                                <button type="button" class="flex h-3.5 w-4 items-center justify-center rounded-sm border border-border text-[0.625rem] leading-none text-text-secondary hover:bg-state-hover hover:text-text-primary" aria-label="Decrease quantity" on:click=move |_| adjust_quantity(rows, &decrease_key, -1)>"−"</button>
                                            </span>
                                        </span>
                                    </td>
                                        <td class="px-1.5 py-3 text-right text-text-primary">{price}</td>
                                        <td class=cash_flow_class>{cash_flow}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
                <footer class="flex items-center border-t border-border px-3 py-3 text-xs">
                <a class="font-semibold text-interactive-text hover:underline" href=options_path.clone()>"＋ Add leg"</a>
            </footer>
    </section>
        }
}

fn format_cash_flow(quantity: i32, instrument: &str, price: &str) -> String {
    let normalized = price.trim().trim_start_matches('$').replace(',', "");
    let Ok(unit_price) = normalized.parse::<f64>() else {
        return "—".to_owned();
    };
    let multiplier =
        if instrument.eq_ignore_ascii_case("CALL") || instrument.eq_ignore_ascii_case("PUT") {
            100.0
        } else {
            1.0
        };
    let value = -(quantity as f64) * unit_price * multiplier;
    format!("${:.2}", value.abs())
}

fn adjust_quantity(rows: RwSignal<Vec<DraftLeg>>, key: &str, magnitude_delta: i32) {
    rows.update(|legs| {
        if let Some(index) = legs.iter().position(|leg| leg.key == key) {
            let sign = if legs[index].quantity < 0 { -1 } else { 1 };
            let magnitude = legs[index].quantity.unsigned_abs() as i32;
            let next_magnitude = (magnitude + magnitude_delta).max(0);
            if next_magnitude == 0 {
                legs.remove(index);
            } else {
                legs[index].quantity = sign * next_magnitude;
            }
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
