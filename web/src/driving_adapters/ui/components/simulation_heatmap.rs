use crate::application::asset_simulation::PnlHeatmap;
use leptos::prelude::*;

#[component]
pub fn SimulationPnlHeatmap(heatmap: PnlHeatmap) -> impl IntoView {
    let columns = heatmap.spot_prices.clone();
    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Mock profit and loss heatmap">
            <div class="panel-header"><div><h2 class="text-sm font-semibold">"P&L Heatmap (USD)"</h2><p class="text-[0.625rem] uppercase tracking-wider text-level-special">"Deterministic fixture"</p></div><span class="text-xs text-text-secondary">"Spot Price at Expiration"</span></div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-auto p-3">
                <table class="w-full min-w-[42rem] border-collapse text-xs numeric">
                    <thead><tr><th class="px-2 py-2 text-left font-medium text-text-secondary">"IV"</th>{columns.into_iter().map(|spot| view! { <th class="px-2 py-2 text-center font-medium text-text-secondary">{format_spot(spot)}</th> }).collect_view()}</tr></thead>
                    <tbody>
                        {heatmap.implied_volatilities.into_iter().zip(heatmap.values).enumerate().map(|(row_index, (volatility, row))| view! {
                            <tr><th class=if row_index == heatmap.selected_row { "px-2 py-2 text-left font-semibold text-interactive-text" } else { "px-2 py-2 text-left font-medium text-text-secondary" }>{format!("{volatility:.1}%")}</th>
                            {row.into_iter().enumerate().map(|(column_index, value)| {
                                let selected = row_index == heatmap.selected_row && column_index == heatmap.selected_column;
                                let cell = if selected { "border border-state-focus bg-state-selected text-text-primary" } else if value > 0.0 { "border border-finance-positive/20 bg-finance-positive/20 text-finance-positive" } else if value < 0.0 { "border border-finance-negative/20 bg-finance-negative/20 text-negative-text" } else { "border border-border bg-canvas text-text-primary" };
                                view! { <td class=format!("px-2 py-2 text-center {cell}")>{format_pnl(value)}</td> }
                            }).collect_view()}</tr>
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </section>
    }
}

fn format_spot(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn format_pnl(value: f64) -> String {
    format!("{value:.0}")
}
