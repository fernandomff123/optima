use super::AssetTabs;
use crate::application::asset_overview::AssetOverviewReadModel;
use leptos::prelude::*;

#[component]
pub fn AssetHeader(model: AssetOverviewReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    let stale = model.is_stale;
    view! {
        <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6">
            <div class="flex min-h-[5.5rem] flex-wrap items-end justify-between gap-4 pb-4">
                <div>
                    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1.5">
                        <h1 class="numeric text-3xl font-black tracking-tight text-text-primary sm:text-[2.125rem]">{model.symbol.clone()}</h1>
                        <span class="text-base font-medium text-text-primary">{model.name}</span>
                        <span class="text-sm text-text-secondary">"· " {model.venue}</span>
                        {model.is_mock.then(|| view! { <span class="mock-indicator">"Mock"</span> })}
                    </div>
                    <div class="mt-1.5 flex flex-wrap items-center gap-x-5 gap-y-2 numeric">
                        <span class="text-2xl font-semibold leading-none text-text-primary sm:text-[1.75rem]">{model.price}</span>
                        <span class=format!("text-base font-semibold {change_class}")>{model.absolute_change} " (" {model.percentage_change} ") " <span class="sr-only">{if model.change_positive { "positive change" } else { "negative change" }}</span></span>
                        <span class="flex items-center gap-2 text-xs font-semibold text-finance-positive"><span class="size-1.5 rounded-full bg-finance-positive"></span>{model.market_status}</span>
                    </div>
                </div>
                <div class="pb-0.5 text-right text-xs leading-5 text-text-muted-readable">
                    <p><time datetime=model.datetime>{model.observed_at}</time> " · " {model.currency}</p>
                    <p class=if stale { "mt-1 font-semibold text-level-special" } else { "mt-1" }>{model.freshness}</p>
                </div>
            </div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities />
        </header>
    }
}
