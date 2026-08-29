use crate::application::asset_options::OptionChainRow;
use leptos::prelude::*;

const HEADINGS: [&str; 7] = ["Last", "Bid", "Ask", "IV", "Delta", "OI", "Volume"];

#[component]
pub fn OptionsChain(rows: Vec<OptionChainRow>) -> impl IntoView {
    view! {
        <div class="dense-scrollbar min-h-0 flex-1 overflow-auto bg-canvas">
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
                    {rows.into_iter().map(|row| {
                        let row_class = if row.is_selected { "border-b border-interactive-source bg-state-selected/45" } else { "border-b border-border hover:bg-state-hover" };
                        let strike_class = if row.is_atm { "border-x border-border bg-level-special/10 px-2 py-2 text-center font-bold text-level-special" } else { "border-x border-border px-2 py-2 text-center font-bold text-text-primary" };
                        view! {
                            <tr class=row_class>
                                <td class="px-2 py-2 font-medium text-finance-positive">{row.call.last}</td><td class="px-2 py-2 text-text-primary">{row.call.bid}</td><td class="px-2 py-2 text-text-primary">{row.call.ask}</td><td class="px-2 py-2 text-text-secondary">{row.call.iv}</td><td class="px-2 py-2 text-text-primary">{row.call.delta}</td><td class="px-2 py-2 text-text-primary">{row.call.open_interest}</td><td class="px-2 py-2 text-text-primary">{row.call.volume}</td>
                                <th class=strike_class scope="row">{row.strike}</th>
                                <td class="px-2 py-2 text-text-primary">{row.put.last}</td><td class="px-2 py-2 text-text-primary">{row.put.bid}</td><td class="px-2 py-2 text-text-primary">{row.put.ask}</td><td class="px-2 py-2 text-text-secondary">{row.put.iv}</td><td class="px-2 py-2 font-medium text-negative-text">{row.put.delta}</td><td class="px-2 py-2 text-text-primary">{row.put.open_interest}</td><td class="px-2 py-2 text-text-primary">{row.put.volume}</td>
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}
