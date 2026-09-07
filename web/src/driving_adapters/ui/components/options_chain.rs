use crate::application::asset_options::{
    OptionChainRow, OptionKind, OptionQuote, OptionSelection, OptionSide,
};
use crate::driving_adapters::ui::simulation_draft::DraftLeg;
use leptos::prelude::*;

const HEADINGS: [&str; 7] = ["Last", "Bid", "Ask", "IV", "Delta", "OI", "Volume"];

#[component]
pub fn OptionsChain(
    rows: Vec<OptionChainRow>,
    draft_legs: ReadSignal<Vec<DraftLeg>>,
    on_preview: Callback<OptionSelection>,
    on_quote: Callback<OptionSelection>,
) -> impl IntoView {
    view! {
        <div class="dense-scrollbar min-h-0 min-w-0 flex-1 overflow-auto bg-canvas">
            <table class="w-full min-w-[62rem] table-fixed border-collapse text-right text-xs numeric" aria-label="Mock AAPL options chain">
                <thead class="sticky top-0 z-10 bg-surface">
                    <tr class="border-b border-border text-[0.6875rem] font-semibold uppercase tracking-wide">
                        <th class="px-2 py-2 text-center text-interactive-text" colspan="7" scope="colgroup">"Calls"</th>
                        <th class="w-20 border-x border-border px-2 py-2 text-center text-text-primary" rowspan="2" scope="col">"Strike"</th>
                        <th class="px-2 py-2 text-center text-negative-text" colspan="7" scope="colgroup">"Puts"</th>
                    </tr>
                    <tr class="border-b border-border text-text-secondary">
                        {HEADINGS.into_iter().map(|value| view! { <th class="px-2 py-2 font-medium" scope="col">{value}</th> }).collect_view()}
                        {HEADINGS.into_iter().map(|value| view! { <th class="px-2 py-2 font-medium" scope="col">{value}</th> }).collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {rows.into_iter().enumerate().map(|(row_index, row)| {
                        let strike = row.strike.clone();
                        let strike_class = if row.is_atm { "border-x border-border bg-level-special/10 px-2 py-2 text-center font-bold text-level-special" } else { "border-x border-border px-2 py-2 text-center font-bold text-text-primary" };
                        view! {
                            <tr class="border-b border-border hover:bg-state-hover/35">
                                <ContractCells row_index kind=OptionKind::Call side=row.call strike=strike.clone() draft_legs on_preview on_quote />
                                <th class=strike_class scope="row">{strike.clone()}</th>
                                <ContractCells row_index kind=OptionKind::Put side=row.put strike draft_legs on_preview on_quote />
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn ContractCells(
    row_index: usize,
    kind: OptionKind,
    side: OptionSide,
    strike: String,
    draft_legs: ReadSignal<Vec<DraftLeg>>,
    on_preview: Callback<OptionSelection>,
    on_quote: Callback<OptionSelection>,
) -> impl IntoView {
    let bid_selection = OptionSelection {
        row_index,
        kind,
        quote: OptionQuote::Bid,
    };
    let ask_selection = OptionSelection {
        row_index,
        kind,
        quote: OptionQuote::Ask,
    };
    let bid_label = format!("Sell {} {} at bid {}", kind.label(), strike, side.bid);
    let ask_label = format!("Buy {} {} at ask {}", kind.label(), strike, side.ask);
    let draft_instrument = kind.label().to_uppercase();
    let draft_strike = strike.clone();
    let draft_quantity: Memo<Option<i32>> = Memo::new(move |_| {
        draft_legs
            .get()
            .into_iter()
            .find(|leg| {
                leg.instrument.eq_ignore_ascii_case(&draft_instrument) && leg.strike == draft_strike
            })
            .map(|leg| leg.quantity)
    });
    let bid_in_draft = move || {
        draft_quantity
            .get()
            .is_some_and(|quantity| quantity.is_negative())
    };
    let ask_in_draft = move || {
        draft_quantity
            .get()
            .is_some_and(|quantity| quantity.is_positive())
    };
    let last_tone = if kind == OptionKind::Call {
        "text-finance-positive"
    } else {
        "text-text-primary"
    };
    let delta_tone = if kind == OptionKind::Put {
        "text-negative-text"
    } else {
        "text-text-primary"
    };
    view! {
        <td class=format!("px-2 py-2 font-medium {last_tone}")>{side.last}</td>
        <td class="p-0">
            <button type="button" class=move || quote_class(bid_in_draft(), OptionQuote::Bid) aria-label=bid_label aria-pressed=bid_in_draft on:mouseenter=move |_| on_preview.run(preview_selection(row_index, kind, OptionQuote::Bid, draft_quantity.get())) on:focus=move |_| on_preview.run(preview_selection(row_index, kind, OptionQuote::Bid, draft_quantity.get())) on:click=move |_| on_quote.run(bid_selection)>{side.bid}</button>
        </td>
        <td class="p-0">
            <button type="button" class=move || quote_class(ask_in_draft(), OptionQuote::Ask) aria-label=ask_label aria-pressed=ask_in_draft on:mouseenter=move |_| on_preview.run(preview_selection(row_index, kind, OptionQuote::Ask, draft_quantity.get())) on:focus=move |_| on_preview.run(preview_selection(row_index, kind, OptionQuote::Ask, draft_quantity.get())) on:click=move |_| on_quote.run(ask_selection)>{side.ask}</button>
        </td>
        <td class="px-2 py-2 text-text-secondary">{side.iv}</td>
        <td class=format!("px-2 py-2 {delta_tone}")>{side.delta}</td>
        <td class="px-2 py-2 text-text-primary">{side.open_interest}</td>
        <td class="px-2 py-2 text-text-primary">{side.volume}</td>
    }
}

fn quote_class(in_draft: bool, quote: OptionQuote) -> &'static str {
    if in_draft {
        match quote {
            OptionQuote::Bid => {
                "min-h-8 w-full border border-negative-text bg-negative-text/25 px-2 py-2 text-right font-semibold text-negative-text"
            }
            OptionQuote::Ask => {
                "min-h-8 w-full border border-interactive-source bg-state-selected px-2 py-2 text-right font-semibold text-interactive-text"
            }
        }
    } else {
        match quote {
            OptionQuote::Bid => {
                "min-h-8 w-full border border-transparent px-2 py-2 text-right text-text-primary hover:border-negative-text/60 hover:bg-negative-text/10 hover:text-negative-text active:border-negative-text active:ring-1 active:ring-negative-text focus-visible:border-negative-text"
            }
            OptionQuote::Ask => {
                "min-h-8 w-full border border-transparent px-2 py-2 text-right text-text-primary hover:border-interactive-source hover:bg-state-hover hover:text-interactive-text active:border-interactive-source active:ring-1 active:ring-interactive-source focus-visible:border-interactive-source"
            }
        }
    }
}

fn preview_selection(
    row_index: usize,
    kind: OptionKind,
    hovered: OptionQuote,
    quantity: Option<i32>,
) -> OptionSelection {
    let quote = match quantity {
        Some(value) if value.is_negative() => OptionQuote::Bid,
        Some(value) if value.is_positive() => OptionQuote::Ask,
        _ => hovered,
    };
    OptionSelection {
        row_index,
        kind,
        quote,
    }
}
