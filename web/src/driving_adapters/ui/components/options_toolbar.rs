use crate::application::asset_options::AssetOptionsReadModel;
use leptos::prelude::*;

#[component]
pub fn OptionsToolbar(model: AssetOptionsReadModel) -> impl IntoView {
    let controls = [
        ("Expiration", model.expiration),
        ("DTE", model.dte),
        ("Strikes", model.strike_range),
        ("View", "Calls + Puts".to_owned()),
    ];
    view! {
        <div class="dense-scrollbar overflow-x-auto border-b border-border bg-canvas" aria-label="Options filters">
            <div class="flex min-w-max items-center gap-3 px-4 py-3 text-xs">
                {controls.into_iter().map(|(label, value)| view! {
                    <label class="flex items-center gap-2 text-text-secondary"><span>{label}</span><select class="h-9 cursor-not-allowed rounded border border-border bg-surface px-3 text-text-primary opacity-90" disabled aria-label=format!("{label} fixed in this mock")><option selected>{value}</option></select></label>
                }).collect_view()}
                <div class="ml-2 flex h-9 overflow-hidden rounded border border-border" aria-label="Displayed option metric">
                    <span class="grid min-w-14 place-items-center bg-state-selected px-4 font-semibold text-text-primary" aria-current="true">"Last"</span>
                    {["IV", "Delta", "OI", "Volume"].into_iter().map(|value| view! { <button class="min-w-14 cursor-not-allowed border-l border-border px-3 text-text-secondary opacity-70" type="button" disabled>{value}</button> }).collect_view()}
                </div>
            </div>
        </div>
    }
}
