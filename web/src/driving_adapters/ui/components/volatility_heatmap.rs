use crate::application::asset_volatility::VolatilityGrid;
use leptos::prelude::*;

#[component]
pub fn VolatilityHeatmap(grid: VolatilityGrid) -> impl IntoView {
    let days = grid.days_to_expiry.clone();
    let rows = grid
        .moneyness
        .iter()
        .copied()
        .zip(grid.implied_volatility_percent.iter().cloned())
        .enumerate()
        .collect::<Vec<_>>();
    let selected_row = grid.selected_moneyness_index;
    let selected_column = grid.selected_expiry_index;
    view! {
        <div class="dense-scrollbar min-h-0 flex-1 overflow-auto p-4">
            <table class="w-full min-w-[48rem] table-fixed border-collapse text-center numeric" aria-label="Implied volatility heatmap by moneyness and days to expiry">
                <thead>
                    <tr class="text-xs text-text-secondary">
                        <th class="w-24 px-2 py-3 text-left font-medium">"Moneyness"</th>
                        {days.into_iter().map(|day| view! { <th class="px-2 py-3 font-medium">{format!("{day} DTE")}</th> }).collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {rows.into_iter().map(|(row_index, (moneyness, values))| {
                        view! {
                            <tr>
                                <th class="px-2 py-3 text-left text-sm font-medium text-text-primary">{format!("{moneyness:.2}")}</th>
                                {values.into_iter().enumerate().map(|(column_index, value)| {
                                    let selected = row_index == selected_row && column_index == selected_column;
                                    let cell_class = if selected { "border border-interactive-text px-2 py-3 text-sm text-text-primary outline outline-1 outline-interactive-text" } else { "border border-border px-2 py-3 text-sm text-text-primary" };
                                    let intensity = ((value - 18.0) / 16.0).clamp(0.0, 1.0);
                                    let strength = 35.0 + intensity * 55.0;
                                    let style = format!("background: color-mix(in srgb, #3B82F6 {strength:.0}%, #173A6F);");
                                    view! {
                                        <td class=cell_class style=style>
                                            {format!("{value:.1}%")}
                                        </td>
                                    }
                                }).collect_view()}
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <div class="mx-auto mt-5 flex max-w-sm items-center gap-3 text-xs text-text-secondary"><span>"18%"</span><span class="h-2 flex-1 bg-gradient-to-r from-[#173A6F] to-[#3B82F6]"></span><span>"34%"</span></div>
        </div>
    }
}
