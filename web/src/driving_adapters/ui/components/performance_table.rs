use super::FinancialValue;
use crate::application::asset_overview::DisplayTable;
use leptos::prelude::*;

#[component]
pub fn PerformanceTable(table: DisplayTable) -> impl IntoView {
    let rows = table
        .rows
        .into_iter()
        .zip(table.tones)
        .zip(table.suffixes)
        .zip(table.units);
    view! {
        <div class="dense-scrollbar overflow-x-auto">
            <table class="w-full min-w-[23rem] table-fixed border-collapse text-left text-sm numeric">
                <caption class="sr-only">{table.title}</caption>
                <colgroup><col class="w-[29%]"/><col class="w-[23.66%]"/><col class="w-[23.66%]"/><col class="w-[23.68%]"/></colgroup>
                <thead><tr>{table.headings.into_iter().enumerate().map(|(index, heading)| {
                    let class = if index == 0 { "table-header text-left" } else { "table-header text-right" };
                    view! { <th class=format!("whitespace-nowrap {class}")>{heading}</th> }
                }).collect_view()}</tr></thead>
                <tbody>{rows.map(|(((row, tones), suffixes), units)| view! { <tr>{row.into_iter().zip(tones).zip(suffixes).zip(units).enumerate().map(|(index, (((value, tone), suffix), unit))| {
                    if index == 0 {
                        view! { <td class="whitespace-nowrap border-b border-border px-3 py-1.5 text-left text-text-secondary">{value.unwrap_or_else(|| "Unavailable".into())}</td> }.into_any()
                    } else {
                        let label = format!("Financial performance column {index}");
                        view! { <td class="border-b border-border px-3 py-1.5 text-right"><FinancialValue value suffix unit tone label /></td> }.into_any()
                    }
                }).collect_view()}</tr> }).collect_view()}</tbody>
            </table>
        </div>
    }
}
