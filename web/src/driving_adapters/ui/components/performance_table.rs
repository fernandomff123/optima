use super::FinancialValue;
use crate::application::asset_overview::DisplayTable;
use leptos::prelude::*;

#[component]
pub fn PerformanceTable(table: DisplayTable) -> impl IntoView {
    let rows = table.rows.into_iter().zip(table.tones).zip(table.units);
    view! {
        <div class="dense-scrollbar overflow-x-auto">
            <table class="w-full min-w-[23rem] border-collapse text-left text-sm numeric">
                <caption class="sr-only">{table.title}</caption>
                <thead><tr>{table.headings.into_iter().enumerate().map(|(index, heading)| {
                    let class = if index == 0 { "table-header text-left" } else { "table-header text-right" };
                    view! { <th class=class>{heading}</th> }
                }).collect_view()}</tr></thead>
                <tbody>{rows.map(|((row, tones), units)| view! { <tr>{row.into_iter().zip(tones).zip(units).enumerate().map(|(index, ((value, tone), unit))| {
                    if index == 0 {
                        view! { <td class="border-b border-border px-3 py-2 text-left text-text-secondary">{value.unwrap_or_else(|| "Unavailable".into())}</td> }.into_any()
                    } else {
                        let label = format!("Financial performance column {index}");
                        view! { <td class="border-b border-border px-3 py-2 text-right"><FinancialValue value unit tone label compact_unit=true /></td> }.into_any()
                    }
                }).collect_view()}</tr> }).collect_view()}</tbody>
            </table>
        </div>
    }
}
