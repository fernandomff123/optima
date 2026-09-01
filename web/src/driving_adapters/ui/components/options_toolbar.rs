use crate::application::asset_options::AssetOptionsReadModel;
use leptos::prelude::*;

#[component]
pub fn OptionsToolbar(
    model: AssetOptionsReadModel,
    underlying_quantity: Memo<i32>,
    on_underlying: Callback<i32>,
) -> impl IntoView {
    let controls = [
        ("Expiration", model.expiration),
        ("DTE", model.dte),
        ("Strikes", model.strike_range),
        ("View", "Calls + Puts".to_owned()),
    ];
    view! {
        <div class="dense-scrollbar overflow-x-auto border-b border-border bg-canvas" aria-label="Options filters and underlying draft">
            <div class="flex min-w-max items-center gap-3 px-4 py-3 text-xs">
                {controls.into_iter().map(|(label, value)| view! {
                    <label class="flex items-center gap-2 text-text-secondary">
                        <span>{label}</span>
                        <select class="h-9 cursor-not-allowed rounded border border-border bg-surface px-3 text-text-primary opacity-90" disabled aria-label=format!("{label} fixed in this mock")>
                            <option selected>{value}</option>
                        </select>
                    </label>
                }).collect_view()}
                <div class="ml-2 flex items-center gap-2" aria-label="Underlying simulation draft">
                    <span class="font-medium text-text-secondary">"Underlying"</span>
                    <div class="flex h-9 overflow-hidden rounded border border-border">
                        <button
                            type="button"
                            class=move || if underlying_quantity.get().is_negative() {
                                "inline-flex min-w-20 items-center justify-center gap-1 border-r border-negative-text bg-negative-text/25 px-3 font-semibold text-negative-text"
                            } else {
                                "inline-flex min-w-20 items-center justify-center gap-1 border-r border-border px-3 font-semibold text-text-secondary hover:bg-negative-text/10 hover:text-negative-text"
                            }
                            aria-label="Sell 100 underlying shares"
                            aria-pressed=move || underlying_quantity.get().is_negative()
                            on:click=move |_| on_underlying.run(-100)
                        ><span aria-hidden="true" class="text-sm">"↓"</span>"Sell 100"</button>
                        <button
                            type="button"
                            class=move || if underlying_quantity.get().is_positive() {
                                "inline-flex min-w-20 items-center justify-center gap-1 bg-state-selected px-3 font-semibold text-interactive-text"
                            } else {
                                "inline-flex min-w-20 items-center justify-center gap-1 px-3 font-semibold text-text-secondary hover:bg-state-hover hover:text-interactive-text"
                            }
                            aria-label="Buy 100 underlying shares"
                            aria-pressed=move || underlying_quantity.get().is_positive()
                            on:click=move |_| on_underlying.run(100)
                        ><span aria-hidden="true" class="text-sm">"↑"</span>"Buy 100"</button>
                    </div>
                    <span class=move || if underlying_quantity.get().is_positive() {
                        "numeric min-w-16 font-semibold text-interactive-text"
                    } else if underlying_quantity.get().is_negative() {
                        "numeric min-w-16 font-semibold text-negative-text"
                    } else {
                        "numeric min-w-16 font-semibold text-text-secondary"
                    }>{move || format!("Qty {:+}", underlying_quantity.get())}</span>
                </div>
            </div>
            <p class="sr-only" aria-live="polite">{move || format!("Underlying draft quantity {:+}", underlying_quantity.get())}</p>
        </div>
    }
}
