use super::AssetTabs;
use crate::application::asset_options::AssetOptionsReadModel;
use leptos::prelude::*;

#[component]
pub fn OptionsAssetHeader(model: AssetOptionsReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    let summary = [
        ("Spot", model.price.clone(), false),
        ("IV Rank", model.iv_rank.clone(), false),
        ("Put/Call OI", model.put_call_oi.clone(), false),
        ("Earnings", model.earnings.clone(), true),
    ];
    view! {
        <header class="border-b border-border bg-canvas px-4 pt-3 sm:px-6">
            <div class="flex min-h-[5.25rem] flex-wrap items-start justify-between gap-5 pb-2">
                <div>
                    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                        <h1 class="numeric text-[1.75rem] font-black leading-none tracking-tight text-text-primary">{model.symbol.clone()}</h1>
                        <span class="text-sm font-medium text-text-primary">{model.name}</span>
                        <span class="text-xs text-text-muted-readable">"•"</span>
                        <span class="text-xs font-medium text-text-secondary">{model.venue}</span>
                    </div>
                    <div class="mt-3 flex items-baseline gap-3 numeric">
                        <span class="text-[1.625rem] font-semibold leading-none text-text-primary">{model.price}</span>
                        <span class=format!("text-sm font-semibold {change_class}")>{model.absolute_change} "  " {model.percentage_change}</span>
                    </div>
                </div>
                <dl class="grid grid-cols-2 gap-x-8 gap-y-3 pt-1 text-xs sm:grid-cols-4 xl:gap-x-12">
                    {summary.into_iter().map(|(label, value, special)| view! {
                        <div>
                            <dt class="text-text-secondary">{label}</dt>
                            <dd class=if special {
                                "numeric mt-1 font-semibold text-level-special"
                            } else {
                                "numeric mt-1 font-semibold text-text-primary"
                            }>{value}</dd>
                        </div>
                    }).collect_view()}
                </dl>
            </div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities />
        </header>
    }
}
