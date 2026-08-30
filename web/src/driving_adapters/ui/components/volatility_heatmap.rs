use crate::application::asset_volatility::VolatilityGrid;
use leptos::prelude::*;

#[component]
pub fn VolatilityHeatmap(grid: VolatilityGrid) -> impl IntoView {
    let minimum = grid
        .implied_volatility_percent
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let maximum = grid
        .implied_volatility_percent
        .iter()
        .flatten()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
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
                                    let tone = match value {
                                        value if value < 21.0 => "bg-state-selected/35",
                                        value if value < 23.0 => "bg-state-selected/55",
                                        value if value < 25.0 => "bg-interactive-source/45",
                                        value if value < 27.0 => "bg-interactive-source/65",
                                        value if value < 29.0 => "bg-state-focus/75",
                                        _ => "bg-state-focus",
                                    };
                                    let selection = if selected { "border-interactive-text outline outline-1 outline-interactive-text" } else { "border-border" };
                                    let cell_class = format!("border px-2 py-3 text-sm text-text-primary {tone} {selection}");
                                    view! {
                                        <td class=cell_class>
                                            {format!("{value:.1}%")}
                                        </td>
                                    }
                                }).collect_view()}
                            </tr>
                        }
                    }).collect_view()}
                </tbody>
            </table>
            <div class="mx-auto mt-5 flex max-w-sm items-center gap-3 text-xs text-text-secondary"><span>{format!("{minimum:.1}%")}</span><span class="h-2 flex-1 bg-gradient-to-r from-state-selected via-interactive-source to-state-focus"></span><span>{format!("{maximum:.1}%")}</span></div>
        </div>
    }
}
