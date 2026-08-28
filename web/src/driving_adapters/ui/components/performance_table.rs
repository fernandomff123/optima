use crate::application::asset_overview::DisplayTable;
use leptos::prelude::*;

#[component]
pub fn PerformanceTable(table: DisplayTable) -> impl IntoView {
    view! {
        <div class="dense-scrollbar overflow-x-auto">
            <table class="w-full min-w-[30rem] border-collapse text-left text-xs numeric">
                <caption class="sr-only">{table.title}</caption>
                <thead><tr>{table.headings.into_iter().map(|heading| view! { <th class="border-b border-border px-3 py-2 font-medium text-text-muted-readable">{heading}</th> }).collect_view()}</tr></thead>
                <tbody>{table.rows.into_iter().map(|row| view! { <tr>{row.into_iter().enumerate().map(|(index, value)| {
                    let value = value.unwrap_or_else(|| "Unavailable".into());
                    let class = if index == 0 { "border-b border-border px-3 py-2 text-text-secondary" } else if value.starts_with('-') { "border-b border-border px-3 py-2 text-negative-text" } else { "border-b border-border px-3 py-2 text-finance-positive" };
                    view! { <td class=class>{value}</td> }
                }).collect_view()}</tr> }).collect_view()}</tbody>
            </table>
        </div>
    }
}
