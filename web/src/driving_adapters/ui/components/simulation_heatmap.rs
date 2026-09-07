use crate::application::asset_simulation::PnlHeatmap;
use leptos::prelude::*;

use super::ScenarioSelection;

#[component]
pub fn SimulationPnlHeatmap(
    heatmap: PnlHeatmap,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    let columns = heatmap.spot_prices.clone();
    let spot_prices = heatmap.spot_prices.clone();
    let implied_volatilities = heatmap.implied_volatilities.clone();
    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Mock profit and loss heatmap">
            <div class="panel-header"><div><h2 class="text-sm font-semibold">"P&L Heatmap (USD)"</h2><p class="text-[0.625rem] uppercase tracking-wider text-level-special">"Deterministic fixture"</p></div><span class="text-xs text-text-secondary">"Spot Price at Expiration"</span></div>
            <div class="dense-scrollbar min-h-0 flex-1 overflow-auto p-3">
                <table class="w-full min-w-[42rem] table-fixed border-collapse text-xs numeric">
                    <thead><tr><th class="w-20 px-2 py-2 text-left font-medium text-text-secondary">"IV"</th>{columns.into_iter().map(|spot| view! { <th class="px-2 py-2 text-center font-medium text-text-secondary">{format_spot(spot)}</th> }).collect_view()}</tr></thead>
                    <tbody>
                        {heatmap.implied_volatilities.into_iter().zip(heatmap.values).enumerate().map(|(row_index, (volatility, row))| {
                            let row_volatilities = implied_volatilities.clone();
                            view! {
                            <tr><th class=move || if row_index == nearest_index(&row_volatilities, selection.get().implied_volatility) { "px-2 py-2 text-left font-semibold text-interactive-text" } else { "px-2 py-2 text-left font-medium text-text-secondary" }>{format!("{volatility:.1}%")}</th>
                            {row.into_iter().enumerate().map(|(column_index, value)| {
                                let cell_spots = spot_prices.clone();
                                let cell_volatilities = implied_volatilities.clone();
                                view! { <td class=move || {
                                    let scenario = selection.get();
                                    let selected = row_index == nearest_index(&cell_volatilities, scenario.implied_volatility)
                                        && column_index == nearest_index(&cell_spots, scenario.spot);
                                    let tone = if selected { "border border-state-focus bg-state-selected text-text-primary" } else if value > 0.0 { "border border-finance-positive/20 bg-finance-positive/20 text-finance-positive" } else if value < 0.0 { "border border-finance-negative/20 bg-finance-negative/20 text-negative-text" } else { "border border-border bg-canvas text-text-primary" };
                                    format!("px-2 py-2 text-center {tone}")
                                }>{format_pnl(value)}</td> }
                            }).collect_view()}</tr>
                        }}).collect_view()}
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

fn nearest_index(values: &[f64], target: f64) -> usize {
    values
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (**left - target).abs().total_cmp(&(**right - target).abs())
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}
