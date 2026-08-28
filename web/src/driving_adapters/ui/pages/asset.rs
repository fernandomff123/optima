use crate::{
    application::read_models::FeedbackState,
    domain::navigation::asset_overview_path,
    driving_adapters::ui::{
        components::{AssetTabs, DataState, Panel},
        plotly::PlotlyHost,
    },
};
use leptos::prelude::*;
use leptos_router::{components::Redirect, hooks::use_params_map};

#[component]
pub fn AssetRedirect() -> impl IntoView {
    let params = use_params_map();
    let ticker = params
        .read()
        .get("ticker")
        .unwrap_or_else(|| "SPX".to_owned());
    view! { <Redirect path=asset_overview_path(&ticker) /> }
}

#[component]
pub fn AssetWorkspacePage(section: &'static str) -> impl IntoView {
    let params = use_params_map();
    let ticker = move || {
        params
            .read()
            .get("ticker")
            .unwrap_or_else(|| "SPX".to_owned())
    };
    view! {
        <div>
            <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6 lg:px-8">
                <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
                    <div class="flex items-center gap-3">
                        <span class="grid size-10 place-items-center rounded border border-border bg-surface-elevated text-sm font-black text-text-primary">{move || ticker().chars().next().unwrap_or('S')}</span>
                        <div><p class="text-lg font-semibold tracking-tight text-text-primary numeric">{ticker}</p><p class="text-xs text-text-muted-readable">"Demonstration asset context · no market data"</p></div>
                    </div>
                    <span class="rounded border border-level-special/60 px-2.5 py-1.5 text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Placeholder"</span>
                </div>
                <AssetTabs ticker=ticker() />
            </header>
            <div class="p-4 sm:p-6 lg:p-8">
                <Panel title=move || format!("{} · {}", ticker(), section) eyebrow="Asset workspace">
                    {if section == "chart" {
                        view! { <PlotlyHost label="Plotly chart host. No financial chart is mounted in this milestone." /> }.into_any()
                    } else { view! { <DataState state=FeedbackState::unavailable() /> }.into_any() }}
                </Panel>
            </div>
        </div>
    }
}

#[component]
pub fn AssetOverviewPage() -> impl IntoView {
    view! { <AssetWorkspacePage section="overview" /> }
}
#[component]
pub fn AssetChartPage() -> impl IntoView {
    view! { <AssetWorkspacePage section="chart" /> }
}
#[component]
pub fn AssetOptionsPage() -> impl IntoView {
    view! { <AssetWorkspacePage section="options" /> }
}
#[component]
pub fn AssetVolatilityPage() -> impl IntoView {
    view! { <AssetWorkspacePage section="volatility" /> }
}
#[component]
pub fn AssetGexPage() -> impl IntoView {
    view! { <AssetWorkspacePage section="gex" /> }
}
#[component]
pub fn AssetSimulationPage() -> impl IntoView {
    view! { <AssetWorkspacePage section="simulation" /> }
}
