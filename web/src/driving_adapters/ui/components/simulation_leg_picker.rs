use crate::{
    application::asset_options::{AssetOptionsReadModel, OptionSelection},
    driving_adapters::ui::simulation_draft::{
        DraftLeg, option_draft_leg, read_draft_legs, underlying_draft_leg,
        upsert_draft_leg_with_quantity,
    },
};
use leptos::prelude::*;

use super::OptionsChain;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegPickerTab {
    Options,
    Underlying,
}

#[component]
pub fn SimulationLegPicker(
    model: AssetOptionsReadModel,
    draft_rows: RwSignal<Vec<DraftLeg>>,
) -> impl IntoView {
    let selected_tab = RwSignal::new(LegPickerTab::Options);
    let action_model = model.clone();
    let symbol = model.symbol.clone();
    let expiration = model.expiration.clone();
    let chain = model.chain.clone();
    let draft_legs = draft_rows.read_only();
    let on_preview = Callback::new(|_: OptionSelection| {});
    let on_quote = Callback::new(move |selection: OptionSelection| {
        let contract = action_model.contract_for(selection);
        if upsert_draft_leg_with_quantity(option_draft_leg(&contract)).is_some() {
            draft_rows.set(read_draft_legs());
        }
    });
    let underlying_key = underlying_draft_leg(&symbol, 100).key;
    let underlying_quantity = Memo::new(move |_| {
        draft_rows
            .get()
            .into_iter()
            .find(|leg| leg.key == underlying_key)
            .map(|leg| leg.quantity)
            .unwrap_or_default()
    });
    let underlying_symbol = symbol.clone();
    let on_underlying = Callback::new(move |quantity: i32| {
        if upsert_draft_leg_with_quantity(underlying_draft_leg(
            &underlying_symbol,
            quantity,
        ))
        .is_some()
        {
            draft_rows.set(read_draft_legs());
        }
    });

    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Add leg to simulation strategy">
            <header class="flex min-h-11 shrink-0 flex-wrap items-center gap-4 border-b border-border px-4 text-xs">
                <h2 class="mr-2 text-sm font-semibold">"Add leg to strategy"</h2>
                <button type="button" class=move || picker_tab_class(selected_tab.get() == LegPickerTab::Options) aria-pressed=move || selected_tab.get() == LegPickerTab::Options on:click=move |_| selected_tab.set(LegPickerTab::Options)>"Options"</button>
                <button type="button" class=move || picker_tab_class(selected_tab.get() == LegPickerTab::Underlying) aria-pressed=move || selected_tab.get() == LegPickerTab::Underlying on:click=move |_| selected_tab.set(LegPickerTab::Underlying)>"Underlying"</button>
                <div class="ml-auto flex items-center gap-2 text-text-secondary">
                    <span>{expiration}</span>
                    <span class="rounded border border-border bg-canvas px-3 py-1.5">"All strikes"</span>
                    <span class="rounded border border-border bg-canvas px-3 py-1.5">"Calls + Puts"</span>
                </div>
            </header>
            {move || match selected_tab.get() {
                LegPickerTab::Options => view! {
                    <OptionsChain rows=chain.clone() draft_legs on_preview on_quote />
                    <footer class="flex shrink-0 items-center justify-between border-t border-border px-4 py-2 text-[0.6875rem] text-text-secondary">
                        <span>"Click Bid to sell · Click Ask to buy"</span>
                        <span class="text-interactive-text">"The browser strategy draft updates instantly."</span>
                    </footer>
                }.into_any(),
                LegPickerTab::Underlying => view! {
                    <div class="flex min-h-40 flex-1 items-center justify-center p-6">
                        <div class="flex flex-col items-center gap-4 rounded border border-border bg-canvas p-5">
                            <div class="text-center"><p class="text-sm font-semibold">{symbol.clone()}</p><p class="mt-1 text-xs text-text-secondary">"Trade the underlying in blocks of 100 shares"</p></div>
                            <div class="flex overflow-hidden rounded border border-border text-xs">
                                <button type="button" class=move || underlying_button_class(underlying_quantity.get().is_negative(), false) on:click=move |_| on_underlying.run(-100)>"Sell 100"</button>
                                <button type="button" class=move || underlying_button_class(underlying_quantity.get().is_positive(), true) on:click=move |_| on_underlying.run(100)>"Buy 100"</button>
                            </div>
                            <p class="numeric text-xs text-text-secondary">{move || format!("Draft quantity {:+}", underlying_quantity.get())}</p>
                        </div>
                    </div>
                }.into_any(),
            }}
        </section>
    }
}

fn picker_tab_class(selected: bool) -> &'static str {
    if selected {
        "h-11 border-b-2 border-interactive-text px-2 font-semibold text-interactive-text"
    } else {
        "h-11 border-b-2 border-transparent px-2 text-text-secondary hover:text-text-primary"
    }
}

fn underlying_button_class(selected: bool, buy: bool) -> &'static str {
    match (selected, buy) {
        (true, true) => "min-h-9 min-w-24 bg-state-selected px-4 font-semibold text-interactive-text",
        (true, false) => "min-h-9 min-w-24 border-r border-negative-text bg-negative-text/25 px-4 font-semibold text-negative-text",
        (false, true) => "min-h-9 min-w-24 px-4 text-text-secondary hover:bg-state-hover hover:text-interactive-text",
        (false, false) => "min-h-9 min-w-24 border-r border-border px-4 text-text-secondary hover:bg-negative-text/10 hover:text-negative-text",
    }
}
