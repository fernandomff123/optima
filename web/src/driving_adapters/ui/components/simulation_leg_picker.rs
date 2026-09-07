use crate::{
    application::asset_options::{
        AssetOptionsReadModel, ContractDetail, OptionKind, OptionQuote, OptionSelection, OptionSide,
    },
    driving_adapters::ui::simulation_draft::{
        DraftLeg, option_draft_leg, read_draft_legs, underlying_draft_leg,
        upsert_draft_leg_with_quantity,
    },
};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LegPickerTab {
    Options,
    Underlying,
}

#[component]
pub fn SimulationLegPicker(
    model: AssetOptionsReadModel,
    draft_rows: RwSignal<Vec<DraftLeg>>,
    start_underlying: bool,
    on_close: Callback<()>,
) -> impl IntoView {
    let selected_tab = RwSignal::new(if start_underlying {
        LegPickerTab::Underlying
    } else {
        LegPickerTab::Options
    });
    let action_model = model.clone();
    let symbol = model.symbol.clone();
    let base_expiration = model.expiration.clone();
    let base_dte = model.dte.clone();
    let expiration_year = base_expiration.split_whitespace().last().unwrap_or("2025");
    let expiration_choices = vec![
        base_expiration.clone(),
        format!("24 May {expiration_year}"),
        format!("21 Jun {expiration_year}"),
    ];
    let quick_expirations = expiration_choices.clone();
    let selected_expiration = RwSignal::new(base_expiration.clone());
    let quote_base_expiration = base_expiration.clone();
    let quote_base_dte = base_dte.clone();
    let atm = model
        .chain
        .iter()
        .position(|row| row.is_atm)
        .unwrap_or(model.chain.len() / 2);
    let start = atm
        .saturating_sub(2)
        .min(model.chain.len().saturating_sub(5));
    let end = (start + 5).min(model.chain.len());
    let visible_rows = model.chain[start..end].to_vec();
    let draft_legs = draft_rows.read_only();
    let on_quote = Callback::new(move |selection: OptionSelection| {
        let expiration = selected_expiration.get();
        let mut contract = action_model.contract_for(selection);
        apply_expiration(
            &mut contract,
            &action_model.symbol,
            &expiration,
            &base_expiration,
            &base_dte,
            selection.kind,
        );
        let mut draft_leg = option_draft_leg(&contract);
        apply_mock_term_premium(&mut draft_leg, &expiration, &base_expiration, &base_dte);
        if upsert_draft_leg_with_quantity(draft_leg).is_some() {
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
        if upsert_draft_leg_with_quantity(underlying_draft_leg(&underlying_symbol, quantity))
            .is_some()
        {
            draft_rows.set(read_draft_legs());
        }
    });

    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Add leg to strategy">
            <header class="flex min-h-11 shrink-0 items-center gap-4 border-b border-border px-4 text-xs">
                <h2 class="mr-2 shrink-0 text-sm font-semibold uppercase tracking-wide">"Add leg to strategy"</h2>
                <button type="button" class=move || picker_tab_class(selected_tab.get() == LegPickerTab::Options) aria-pressed=move || selected_tab.get() == LegPickerTab::Options on:click=move |_| selected_tab.set(LegPickerTab::Options)>"Options"</button>
                <button type="button" class=move || picker_tab_class(selected_tab.get() == LegPickerTab::Underlying) aria-pressed=move || selected_tab.get() == LegPickerTab::Underlying on:click=move |_| selected_tab.set(LegPickerTab::Underlying)>"Underlying"</button>
                <div class="ml-8 flex min-w-0 items-center gap-2 text-text-secondary">
                    <label class="sr-only" for="simulation-expiration">"Expiration"</label>
                    <select id="simulation-expiration" class="min-h-8 whitespace-nowrap rounded border border-border bg-canvas px-3 text-text-primary" prop:value=move || selected_expiration.get() on:change=move |event| selected_expiration.set(event_target_value(&event))>
                        {expiration_choices.into_iter().map(|expiration| view! { <option value=expiration.clone()>{expiration.clone()}</option> }).collect_view()}
                    </select>
                    {quick_expirations.into_iter().map(|expiration| {
                        let selected_value = expiration.clone();
                        let clicked_value = expiration.clone();
                        let label = short_expiration(&expiration);
                        view! { <button type="button" class=move || expiration_button_class(selected_expiration.get() == selected_value) aria-pressed=move || selected_expiration.get() == expiration on:click=move |_| selected_expiration.set(clicked_value.clone())>{label}</button> }
                    }).collect_view()}
                    <span class="whitespace-nowrap rounded border border-border bg-canvas px-3 py-1.5">"All Strikes"</span>
                    <span class="whitespace-nowrap rounded border border-border bg-canvas px-3 py-1.5">"Calls & Puts"</span>
                    <button type="button" class="ml-2 inline-flex items-center gap-2 whitespace-nowrap px-2 text-text-primary hover:text-interactive-text" aria-label="Close strategy editor" on:click=move |_| on_close.run(())><span>"Close"</span><span class="text-lg leading-none">"×"</span></button>
                </div>
            </header>
            {move || match selected_tab.get() {
                LegPickerTab::Options => view! {
                    <div class="dense-scrollbar min-h-0 flex-1 overflow-auto bg-canvas">
                        <table class="w-full min-w-[68rem] table-fixed border-collapse text-right text-[0.6875rem] numeric" aria-label="Compact simulation option strikes">
                            <thead class="sticky top-0 z-10 bg-surface">
                                <tr class="border-b border-border text-[0.6875rem] font-semibold uppercase tracking-wide"><th class="px-2 py-1.5 text-left text-interactive-text" colspan="6">"Calls"</th><th class="w-20 border-x border-border px-2 py-1.5 text-center text-text-primary" rowspan="2">"Strike"</th><th class="px-2 py-1.5 text-left text-negative-text" colspan="6">"Puts"</th></tr>
                                <tr class="border-b border-border text-text-secondary"><th class="px-2 py-1.5 font-medium">"IV"</th><th class="px-2 py-1.5 font-medium">"Delta"</th><th class="px-2 py-1.5 font-medium">"OI"</th><th class="px-2 py-1.5 font-medium">"Volume"</th><th class="px-2 py-1.5 font-medium text-negative-text">"Sell / Bid"</th><th class="px-2 py-1.5 font-medium text-finance-positive">"Buy / Ask"</th><th class="px-2 py-1.5 font-medium text-finance-positive">"Buy / Ask"</th><th class="px-2 py-1.5 font-medium text-negative-text">"Sell / Bid"</th><th class="px-2 py-1.5 font-medium">"Volume"</th><th class="px-2 py-1.5 font-medium">"OI"</th><th class="px-2 py-1.5 font-medium">"Delta"</th><th class="px-2 py-1.5 font-medium">"IV"</th></tr>
                            </thead>
                            <tbody>{visible_rows.clone().into_iter().enumerate().map(|(offset, row)| { let row_index = start + offset; let strike = row.strike.clone(); let strike_class = if row.is_atm { "border-x border-border bg-state-selected px-2 py-1 text-center font-bold text-interactive-text" } else { "border-x border-border px-2 py-1 text-center font-bold text-text-primary" }; view! { <tr class="border-b border-border hover:bg-state-hover/35"><CompactSide row_index kind=OptionKind::Call side=row.call strike=strike.clone() draft_legs on_quote selected_expiration=selected_expiration.read_only() base_expiration=quote_base_expiration.clone() base_dte=quote_base_dte.clone() /><th class=strike_class>{strike.clone()}</th><CompactSide row_index kind=OptionKind::Put side=row.put strike draft_legs on_quote selected_expiration=selected_expiration.read_only() base_expiration=quote_base_expiration.clone() base_dte=quote_base_dte.clone() /></tr> } }).collect_view()}</tbody>
                        </table>
                    </div>
                    <footer class="grid min-h-11 shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-6 border-t border-border px-4 text-[0.6875rem] text-text-secondary">
                        <span>"Click Bid = Sell  ·  Click Ask = Buy"</span>
                        <span class="flex items-center gap-2"><span>"Quantity"</span><span class="rounded border border-border bg-canvas px-5 py-1.5 text-text-primary">"1"</span><span>"Price"</span><span class="rounded border border-border bg-canvas px-5 py-1.5 text-text-primary">"Market side"</span></span>
                        <span class="justify-self-end text-interactive-text">"Selecting a price updates the strategy instantly."</span>
                    </footer>
                }.into_any(),
                LegPickerTab::Underlying => view! {
                    <div class="flex min-h-40 flex-1 items-center justify-center bg-canvas p-6"><div class="flex items-center gap-5 rounded border border-border bg-surface p-5"><div><p class="text-sm font-semibold">{symbol.clone()}</p><p class="mt-1 text-xs text-text-secondary">"Trade the underlying in blocks of 100 shares"</p></div><div class="flex overflow-hidden rounded border border-border text-xs"><button type="button" class=move || underlying_button_class(underlying_quantity.get().is_negative(), false) on:click=move |_| on_underlying.run(-100)>"Sell 100"</button><button type="button" class=move || underlying_button_class(underlying_quantity.get().is_positive(), true) on:click=move |_| on_underlying.run(100)>"Buy 100"</button></div><p class="numeric text-xs text-text-secondary">{move || format!("Draft quantity {:+}", underlying_quantity.get())}</p></div></div>
                }.into_any(),
            }}
        </section>
    }
}

#[component]
fn CompactSide(
    row_index: usize,
    kind: OptionKind,
    side: OptionSide,
    strike: String,
    draft_legs: ReadSignal<Vec<DraftLeg>>,
    on_quote: Callback<OptionSelection>,
    selected_expiration: ReadSignal<String>,
    base_expiration: String,
    base_dte: String,
) -> impl IntoView {
    let bid = OptionSelection {
        row_index,
        kind,
        quote: OptionQuote::Bid,
    };
    let ask = OptionSelection {
        row_index,
        kind,
        quote: OptionQuote::Ask,
    };
    let instrument = kind.label().to_uppercase();
    let tracked_strike = strike.clone();
    let quantity = Memo::new(move |_| {
        draft_legs
            .get()
            .into_iter()
            .find(|leg| {
                leg.instrument.eq_ignore_ascii_case(&instrument) && leg.strike == tracked_strike
            })
            .map(|leg| leg.quantity)
    });
    let bid_selected = move || quantity.get().is_some_and(|value| value.is_negative());
    let ask_selected = move || quantity.get().is_some_and(|value| value.is_positive());
    let delta_class = if kind == OptionKind::Put {
        "px-2 py-1.5 text-negative-text"
    } else {
        "px-2 py-1.5 text-text-primary"
    };
    let bid_value = side.bid.clone();
    let ask_value = side.ask.clone();
    let bid_expiration = base_expiration.clone();
    let ask_expiration = base_expiration.clone();
    let bid_dte = base_dte.clone();
    let ask_dte = base_dte.clone();
    let metrics = view! { <td class="px-2 py-1.5 text-text-secondary">{side.iv.clone()}</td><td class=delta_class>{side.delta.clone()}</td><td class="px-2 py-1.5 text-text-primary">{side.open_interest.clone()}</td><td class="px-2 py-1.5 text-text-primary">{side.volume.clone()}</td> };
    let reverse_metrics = view! { <td class="px-2 py-1.5 text-text-primary">{side.volume}</td><td class="px-2 py-1.5 text-text-primary">{side.open_interest}</td><td class=delta_class>{side.delta}</td><td class="px-2 py-1.5 text-text-secondary">{side.iv}</td> };
    let buttons = view! { <td class="p-1"><button type="button" class=move || quote_class(bid_selected(), false) aria-pressed=bid_selected on:click=move |_| on_quote.run(bid)>{move || mock_term_quote(&bid_value, &selected_expiration.get(), &bid_expiration, &bid_dte)}</button></td><td class="p-1"><button type="button" class=move || quote_class(ask_selected(), true) aria-pressed=ask_selected on:click=move |_| on_quote.run(ask)>{move || mock_term_quote(&ask_value, &selected_expiration.get(), &ask_expiration, &ask_dte)}</button></td> };
    let reverse_bid_value = side.bid.clone();
    let reverse_ask_value = side.ask.clone();
    let reverse_bid_expiration = base_expiration.clone();
    let reverse_ask_expiration = base_expiration;
    let reverse_bid_dte = base_dte.clone();
    let reverse_ask_dte = base_dte;
    let reverse_buttons = view! { <td class="p-1"><button type="button" class=move || quote_class(ask_selected(), true) aria-pressed=ask_selected on:click=move |_| on_quote.run(ask)>{move || mock_term_quote(&reverse_ask_value, &selected_expiration.get(), &reverse_ask_expiration, &reverse_ask_dte)}</button></td><td class="p-1"><button type="button" class=move || quote_class(bid_selected(), false) aria-pressed=bid_selected on:click=move |_| on_quote.run(bid)>{move || mock_term_quote(&reverse_bid_value, &selected_expiration.get(), &reverse_bid_expiration, &reverse_bid_dte)}</button></td> };
    if kind == OptionKind::Call {
        view! { {metrics}{buttons} }.into_any()
    } else {
        view! { {reverse_buttons}{reverse_metrics} }.into_any()
    }
}

fn quote_class(selected: bool, buy: bool) -> &'static str {
    match (selected, buy) {
        (true, true) => {
            "min-h-7 w-full rounded-sm border border-interactive-source bg-state-selected px-2 font-semibold text-interactive-text"
        }
        (true, false) => {
            "min-h-7 w-full rounded-sm border border-negative-text bg-negative-text/25 px-2 font-semibold text-negative-text"
        }
        (false, true) => {
            "min-h-7 w-full rounded-sm border border-finance-positive/60 px-2 text-finance-positive hover:bg-state-hover"
        }
        (false, false) => {
            "min-h-7 w-full rounded-sm border border-negative-text/70 px-2 text-negative-text hover:bg-negative-text/10"
        }
    }
}

fn apply_expiration(
    contract: &mut ContractDetail,
    symbol: &str,
    expiration: &str,
    base_expiration: &str,
    base_dte: &str,
    kind: OptionKind,
) {
    let strike = contract
        .facts
        .iter()
        .find(|(label, _)| label == "Strike")
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| "—".to_owned());
    replace_contract_fact(&mut contract.facts, "Expiration", expiration);
    replace_contract_fact(
        &mut contract.facts,
        "DTE",
        &expiration_dte(expiration, base_expiration, base_dte),
    );
    contract.title = format!(
        "{} {} {} {}",
        symbol,
        expiration.to_uppercase(),
        strike,
        kind.label().to_uppercase(),
    );
}

fn replace_contract_fact(facts: &mut [(String, String)], label: &str, value: &str) {
    if let Some((_, current)) = facts.iter_mut().find(|(candidate, _)| candidate == label) {
        *current = value.to_owned();
    }
}

fn expiration_dte(expiration: &str, base_expiration: &str, base_dte: &str) -> String {
    let base = base_dte.parse::<i32>().unwrap_or_default();
    if expiration == base_expiration {
        base_dte.to_owned()
    } else if expiration.starts_with("24 May") {
        (base + 7).to_string()
    } else if expiration.starts_with("21 Jun") {
        (base + 35).to_string()
    } else {
        base_dte.to_owned()
    }
}

fn apply_mock_term_premium(
    leg: &mut DraftLeg,
    expiration: &str,
    base_expiration: &str,
    base_dte: &str,
) {
    let Ok(base_price) = leg.price.trim().trim_start_matches('$').parse::<f64>() else {
        return;
    };
    leg.price = format!(
        "{:.2}",
        base_price + mock_term_premium(expiration, base_expiration, base_dte)
    );
}

fn mock_term_quote(price: &str, expiration: &str, base_expiration: &str, base_dte: &str) -> String {
    let Ok(base_price) = price.trim().trim_start_matches('$').parse::<f64>() else {
        return price.to_owned();
    };
    format!(
        "{:.2}",
        base_price + mock_term_premium(expiration, base_expiration, base_dte)
    )
}

fn mock_term_premium(expiration: &str, base_expiration: &str, base_dte: &str) -> f64 {
    let base_days = base_dte.parse::<i32>().unwrap_or_default();
    let selected_days = expiration_dte(expiration, base_expiration, base_dte)
        .parse::<i32>()
        .unwrap_or(base_days);
    // Deterministic fixture: later expirations retain more time value.
    // Four cents per extra day keeps the mock chain monotonic by maturity.
    f64::from((selected_days - base_days).max(0)) * 0.04
}

fn short_expiration(expiration: &str) -> String {
    expiration
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn expiration_button_class(selected: bool) -> &'static str {
    if selected {
        "hidden min-h-8 rounded border border-interactive-source bg-state-selected px-3 font-semibold text-interactive-text 2xl:block"
    } else {
        "hidden min-h-8 rounded border border-border px-3 text-text-secondary hover:bg-state-hover hover:text-text-primary 2xl:block"
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
        (true, true) => {
            "min-h-9 min-w-24 bg-state-selected px-4 font-semibold text-interactive-text"
        }
        (true, false) => {
            "min-h-9 min-w-24 border-r border-negative-text bg-negative-text/25 px-4 font-semibold text-negative-text"
        }
        (false, true) => {
            "min-h-9 min-w-24 px-4 text-text-secondary hover:bg-state-hover hover:text-interactive-text"
        }
        (false, false) => {
            "min-h-9 min-w-24 border-r border-border px-4 text-text-secondary hover:bg-negative-text/10 hover:text-negative-text"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiration_shortcuts_keep_full_dates_and_mock_dte_aligned() {
        assert_eq!(short_expiration("17 May 2025"), "17 May");
        assert_eq!(expiration_dte("17 May 2025", "17 May 2025", "36"), "36");
        assert_eq!(expiration_dte("24 May 2025", "17 May 2025", "36"), "43");
        assert_eq!(expiration_dte("21 Jun 2025", "17 May 2025", "36"), "71");
    }
}
