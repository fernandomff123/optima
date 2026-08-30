use crate::driving_adapters::ui::simulation_draft::{
    contains_draft_leg, underlying_draft_leg, upsert_draft_leg,
};
use leptos::prelude::*;

#[component]
pub fn ChartSimulationAction(symbol: String) -> impl IntoView {
    let (quantity, set_quantity) = signal(100_u32);
    let (long_position, set_long_position) = signal(true);
    let draft_key = underlying_draft_leg(&symbol, 100).key;
    let (added, set_added) = signal(contains_draft_leg(&draft_key));
    let direction_button = move |label: &'static str, long: bool| {
        view! {
            <button
                type="button"
                class=move || if long_position.get() == long {
                    "min-h-9 flex-1 rounded border border-interactive-source bg-state-selected px-2 text-xs font-semibold text-interactive-text"
                } else {
                    "min-h-9 flex-1 rounded border border-border bg-canvas px-2 text-xs text-text-secondary hover:bg-state-hover hover:text-text-primary"
                }
                aria-pressed=move || long_position.get() == long
                on:click=move |_| {
                    set_long_position.set(long);
                    set_added.set(false);
                }
            >{label}</button>
        }
    };
    let storage_symbol = symbol.clone();
    let confirmation_symbol = symbol.clone();
    let simulation_path = format!("/assets/{symbol}/simulation");
    view! {
        <section class="mt-2 border-t border-border p-3" aria-label="Underlying simulation draft">
            <div class="flex items-start justify-between gap-3">
                <div>
                    <h2 class="text-sm font-semibold text-text-primary">"Underlying Simulation"</h2>
                    <p class="mt-1 text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Mock interaction"</p>
                </div>
                <span class="numeric text-xs font-semibold text-text-primary">{symbol}</span>
            </div>
            <div class="mt-3 grid grid-cols-[minmax(0,1fr)_7.5rem] gap-2">
                <label class="text-xs text-text-secondary">
                    "Quantity"
                    <input
                        type="number"
                        min="100"
                        step="100"
                        prop:value=move || quantity.get().to_string()
                        class="mt-2 min-h-9 w-full rounded border border-border bg-canvas px-3 text-right text-xs text-text-primary outline-none focus:border-state-focus"
                        on:change=move |event| {
                            if let Ok(value) = event_target_value(&event).parse::<u32>() {
                                let normalized = value.max(100).saturating_add(99) / 100 * 100;
                                set_quantity.set(normalized);
                                set_added.set(false);
                            }
                        }
                    />
                </label>
                <div>
                    <p class="text-xs text-text-secondary">"Direction"</p>
                    <div class="mt-2 flex gap-1">
                        {direction_button("Long", true)}
                        {direction_button("Short", false)}
                    </div>
                </div>
            </div>
            <button
                type="button"
                class=move || if added.get() {
                    "mt-3 min-h-10 w-full rounded border border-interactive-source bg-state-selected px-3 text-xs font-semibold text-interactive-text"
                } else {
                    "mt-3 min-h-10 w-full rounded border border-interactive-source bg-interactive-source px-3 text-xs font-semibold text-white hover:brightness-110"
                }
                aria-label="Add underlying to Simulation"
                aria-pressed=move || added.get()
                disabled=move || added.get()
                on:click=move |_| {
                    let magnitude = i32::try_from(quantity.get()).unwrap_or(i32::MAX);
                    let signed_quantity = if long_position.get() {
                        magnitude
                    } else {
                        -magnitude
                    };
                    if upsert_draft_leg(underlying_draft_leg(&storage_symbol, signed_quantity)) {
                        set_added.set(true);
                    }
                }
            >
                {move || if added.get() {
                    "Added to Simulation".to_owned()
                } else {
                    format!("Add {} to Simulation", if long_position.get() { "Long" } else { "Short" })
                }}
            </button>
            <p class="sr-only" aria-live="polite">
                {move || added.get().then(|| format!(
                    "{} {} units of {} added to the simulation draft",
                    if long_position.get() { "Long" } else { "Short" },
                    quantity.get(),
                    confirmation_symbol,
                ))}
            </p>
            <a class=move || if added.get() { "mt-2 block min-h-9 rounded border border-border px-3 py-2 text-center text-xs font-semibold text-interactive-text hover:bg-state-hover" } else { "hidden" } href=simulation_path>"Open Simulation"</a>
        </section>
    }
}
