use crate::application::asset_simulation::SimulationLeg;
use leptos::prelude::*;

#[component]
pub fn SimulationPosition(strategy_name: String, legs: Vec<SimulationLeg>) -> impl IntoView {
    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Mock simulation position">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Position"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Mock"</span></div>
            <div class="p-3">
                <label class="sr-only" for="simulation-strategy">"Strategy"</label>
                <select id="simulation-strategy" class="min-h-9 w-full rounded border border-border bg-canvas px-3 text-xs text-text-primary" disabled>
                    <option>{strategy_name}</option>
                </select>
            </div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-x-auto">
                <table class="w-full min-w-[17rem] text-xs">
                    <thead class="border-y border-border bg-canvas text-text-secondary"><tr><th class="px-2 py-2 text-left font-medium">"Leg"</th><th class="px-2 py-2 text-left font-medium">"Type"</th><th class="px-2 py-2 text-right font-medium">"Strike"</th><th class="px-2 py-2 text-left font-medium">"Exp"</th><th class="px-2 py-2 text-right font-medium">"Qty"</th><th class="px-2 py-2 text-right font-medium">"Price"</th></tr></thead>
                    <tbody class="divide-y divide-border numeric">
                        {legs.into_iter().map(|leg| {
                            let positive = leg.quantity > 0;
                            view! {
                                <tr class="hover:bg-state-hover">
                                    <td class=if positive { "px-2 py-3 font-semibold text-finance-positive" } else { "px-2 py-3 font-semibold text-negative-text" }>{if positive { "+1" } else { "-1" }}</td>
                                    <td class=if positive { "px-2 py-3 font-semibold text-finance-positive" } else { "px-2 py-3 font-semibold text-negative-text" }>{leg.option_type}</td>
                                    <td class="px-2 py-3 text-right text-text-primary">{leg.strike}</td>
                                    <td class="px-2 py-3 text-text-primary">{leg.expiration}</td>
                                    <td class="px-2 py-3 text-right text-text-primary">{leg.quantity.unsigned_abs()}</td>
                                    <td class="px-2 py-3 text-right text-text-primary">{leg.price}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
            <footer class="flex items-center justify-between border-t border-border px-3 py-3 text-xs">
                <button type="button" class="text-interactive-text opacity-60" disabled title="Requires a simulation draft contract">"＋ Add leg"</button>
                <button type="button" class="text-interactive-text opacity-60" disabled title="Requires a simulation draft contract">"Edit legs"</button>
            </footer>
        </section>
    }
}
