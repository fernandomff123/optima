use crate::{
    application::{
        asset_overview::{AssetOverviewReadModel, AssetOverviewState},
        read_models::FeedbackState,
    },
    composition::asset_overview_use_case,
    domain::navigation::asset_overview_path,
    driving_adapters::ui::{
        components::{
            AssetHeader, AssetTabs, DataState, FactTable, MetricStrip, Panel, PerformanceTable,
        },
        plotly::{AssetOverviewChart, PlotlyHost},
    },
    ports::asset_overview::OverviewScenario,
};
use leptos::prelude::*;
use leptos_router::{
    components::Redirect,
    hooks::{use_params_map, use_query_map},
};

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
                    <div><p class="text-lg font-semibold tracking-tight text-text-primary numeric">{ticker}</p><p class="text-xs text-text-muted-readable">"Demonstration asset context · no market data"</p></div>
                    <span class="rounded border border-level-special/60 px-2.5 py-1.5 text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Placeholder"</span>
                </div>
                <AssetTabs ticker=ticker() />
            </header>
            <div class="p-4 sm:p-6 lg:p-8"><Panel title=move || format!("{} · {}", ticker(), section) eyebrow="Asset workspace">
                {if section == "chart" { view! { <PlotlyHost label="Plotly chart host. No financial chart is mounted in this milestone." /> }.into_any() }
                else { view! { <DataState state=FeedbackState::unavailable() /> }.into_any() }}
            </Panel></div>
        </div>
    }
}

#[component]
pub fn AssetOverviewPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let (retried, set_retried) = signal(false);
    let state = move || {
        let ticker = params
            .read()
            .get("ticker")
            .unwrap_or_else(|| "SPX".to_owned());
        let requested = OverviewScenario::from_query(query.read().get("scenario").as_deref());
        let scenario = if retried.get() && requested == OverviewScenario::RecoverableError {
            OverviewScenario::Normal
        } else {
            requested
        };
        asset_overview_use_case().execute(&ticker, scenario)
    };
    view! {
        {move || match state() {
            AssetOverviewState::Loading => view! { <OverviewSkeleton /> }.into_any(),
            AssetOverviewState::Ready(model) => view! { <OverviewContent model partial=false /> }.into_any(),
            AssetOverviewState::Partial(model) => view! { <OverviewContent model partial=true /> }.into_any(),
            AssetOverviewState::Unavailable { symbol } => view! { <ContextState symbol state=FeedbackState::unavailable() retry=None /> }.into_any(),
            AssetOverviewState::RecoverableError { symbol } => {
                let retry = Callback::new(move |_| set_retried.set(true));
                view! { <ContextState symbol state=FeedbackState::recoverable_error() retry=Some(retry) /> }.into_any()
            },
            AssetOverviewState::TerminalError { symbol } => view! { <ContextState symbol state=FeedbackState::terminal_error() retry=None /> }.into_any(),
        }}
    }
}

#[component]
fn OverviewContent(model: AssetOverviewReadModel, partial: bool) -> impl IntoView {
    let metrics = model.metrics.clone();
    let chart = model.chart.clone();
    let key_statistics = model.key_statistics.clone();
    let performance = model.performance.clone();
    let index_facts = model.index_facts.clone();
    let options_snapshot = model.options_snapshot.clone();
    view! {
        <AssetHeader model=model.clone() />
        <MetricStrip metrics />
        {model.is_stale.then(|| view! { <div class="border-b border-level-special/40 bg-level-special/10 px-4 py-2 text-xs text-level-special sm:px-6 lg:px-8" role="status">"Stale mock snapshot · values remain visible for context."</div> })}
        {partial.then(|| view! { <div class="border-b border-interactive-source/50 bg-state-selected/30 px-4 py-2 text-xs text-interactive-text sm:px-6 lg:px-8" role="status">"Partial snapshot · unavailable fields are identified without replacing valid data."</div> })}
        <div class="space-y-2 p-3 sm:p-4 lg:p-2">
            <div class="grid items-stretch gap-2 xl:grid-cols-[minmax(0,1.83fr)_minmax(20rem,1fr)]">
                <Panel title=format!("{} Price & Volume · 1D", model.symbol) compact=true><AssetOverviewChart chart /></Panel>
                <Panel title="Key Statistics" compact=true><FactTable metrics=key_statistics /></Panel>
            </div>
            <div class="grid items-stretch gap-2 lg:grid-cols-2 2xl:grid-cols-[1.28fr_0.92fr_1fr]">
                <Panel title="Performance" compact=true><PerformanceTable table=performance /></Panel>
                <Panel title="Index Facts" compact=true><FactTable metrics=index_facts /></Panel>
                <Panel title="Options Snapshot" compact=true><FactTable metrics=options_snapshot /></Panel>
            </div>
        </div>
    }
}

#[component]
fn ContextState(
    symbol: String,
    state: FeedbackState,
    retry: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6 lg:px-8"><div class="mb-4 flex items-center gap-3"><h1 class="text-2xl font-black numeric">{symbol.clone()}</h1><span class="mock-indicator">"Mock"</span></div><AssetTabs ticker=symbol /></header>
        <div class="p-4 sm:p-6 lg:p-8"><Panel title="Asset Overview">
            <DataState state />
            {retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text hover:bg-state-hover" on:click=move |_| action.run(())>"Retry locally"</button></div> })}
        </Panel></div>
    }
}

#[component]
fn OverviewSkeleton() -> impl IntoView {
    view! {
        <div aria-busy="true" aria-label="Loading asset overview">
            <div class="border-b border-border bg-surface px-4 py-6 sm:px-6 lg:px-8"><div class="h-8 w-56 animate-pulse rounded bg-surface-elevated"></div><div class="mt-3 h-5 w-80 max-w-full animate-pulse rounded bg-surface-elevated"></div></div>
            <div class="grid gap-3 p-4 sm:p-6 lg:p-8 xl:grid-cols-[minmax(0,2fr)_minmax(18rem,0.9fr)]"><div class="h-[28rem] animate-pulse rounded-panel border border-border bg-surface"></div><div class="h-[28rem] animate-pulse rounded-panel border border-border bg-surface"></div></div>
        </div>
    }
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
