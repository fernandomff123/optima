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
        <header class="border-b border-border bg-surface px-4 pt-4 sm:px-6 lg:px-8">
            <div class="flex flex-wrap items-start justify-between gap-4 pb-4">
                <div>
                    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                        <h1 class="numeric text-2xl font-black tracking-tight text-text-primary sm:text-3xl">{model.symbol.clone()}</h1>
                        <span class="text-sm font-medium text-text-primary">{model.name}</span>
                        <span class="text-xs text-text-secondary">"· " {model.venue}</span>
                        {model.is_mock.then(|| view! { <span class="rounded border border-level-special/60 px-2 py-0.5 text-[0.625rem] font-bold uppercase tracking-wider text-level-special">"Mock"</span> })}
                    </div>
                    <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-2 numeric">
                        <span class="text-xl font-semibold text-text-primary">{model.price}</span>
                        <span class=format!("text-sm font-semibold {change_class}")>{model.absolute_change} " (" {model.percentage_change} ") " <span class="sr-only">{if model.change_positive { "positive change" } else { "negative change" }}</span></span>
                        <span class="flex items-center gap-2 text-[0.6875rem] font-semibold text-finance-positive"><span class="size-1.5 rounded-full bg-finance-positive"></span>{model.market_status}</span>
                    </div>
                </div>
                <div class="text-right text-[0.6875rem] text-text-muted-readable">
                    <p><time datetime=model.datetime>{model.observed_at}</time> " · " {model.currency}</p>
                    <p class=if stale { "mt-1 font-semibold text-level-special" } else { "mt-1" }>{model.freshness}</p>
                </div>
            </div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities />
        </header>
    }
}
