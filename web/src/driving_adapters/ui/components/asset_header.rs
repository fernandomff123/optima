use super::{AssetTabs, ShellIcon, ShellIconKind};
use crate::application::asset_overview::AssetOverviewReadModel;
use leptos::prelude::*;

#[component]
pub fn AssetHeader(model: AssetOverviewReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    view! {
        <header class="border-b border-border bg-canvas px-6 pt-4">
            <div class="flex min-h-[5.75rem] items-start justify-between gap-4 pb-3">
                <div>
                    <div class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
                        <h1 class="numeric text-[2.125rem] font-black leading-none tracking-tight text-text-primary">{model.symbol.clone()}</h1>
                        <span class="text-base font-medium text-text-primary">{model.name}</span>
                        <span class="text-sm text-text-muted-readable">"•"</span>
                        <span class="text-sm font-medium text-text-secondary">{model.venue}</span>
                    </div>
                    <div class="mt-3 flex flex-wrap items-center gap-x-5 gap-y-2 numeric">
                        <span class="text-[1.75rem] font-semibold leading-none text-text-primary">{model.price}</span>
                        <span class=format!("text-base font-semibold {change_class}")>{model.absolute_change} " (" {model.percentage_change} ")" <span class="sr-only">{if model.change_positive { "positive change" } else { "negative change" }}</span></span>
                        <span class="h-5 border-l border-border" aria-hidden="true"></span>
                        <span class="flex items-center gap-2 text-xs font-semibold text-finance-positive"><span class="size-1.5 rounded-full bg-finance-positive"></span>{model.market_status}</span>
                    </div>
                </div>
                <div class="flex items-center gap-3 pt-3 text-text-secondary">
                    <button class="grid size-9 cursor-not-allowed place-items-center opacity-70" type="button" aria-label="Add AAPL to watchlist unavailable" title="Add to watchlist unavailable" disabled><ShellIcon kind=ShellIconKind::Star class="size-6" /></button>
                    <button class="grid size-9 cursor-not-allowed place-items-center opacity-70" type="button" aria-label="More asset actions unavailable" title="More actions unavailable" disabled><ShellIcon kind=ShellIconKind::More class="size-6" /></button>
                </div>
            </div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities />
        </header>
    }
}
