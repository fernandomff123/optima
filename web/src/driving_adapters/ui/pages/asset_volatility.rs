use crate::{
    application::{
        asset_volatility::{AssetVolatilityReadModel, AssetVolatilityState},
        read_models::FeedbackState,
    },
    composition::asset_volatility_use_case,
    driving_adapters::ui::{
        components::{AssetTabs, DataState, Panel, VolatilityHeatmap, VolatilitySnapshot},
        plotly::{VolatilityAnalytics, VolatilitySurfaceChart},
    },
    ports::asset_volatility::VolatilityScenario,
};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VolatilityView {
    Surface,
    Heatmap,
}

#[component]
pub fn AssetVolatilityPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let (retried, set_retried) = signal(false);
    let state = move || {
        let ticker = params
            .read()
            .get("ticker")
            .unwrap_or_else(|| "AAPL".to_owned());
        let requested = VolatilityScenario::from_query(query.read().get("scenario").as_deref());
        let scenario = if retried.get() && requested == VolatilityScenario::RecoverableError {
            VolatilityScenario::Normal
        } else {
            requested
        };
        asset_volatility_use_case().execute(&ticker, scenario)
    };
    view! {{move || match state() {
        AssetVolatilityState::Loading => view! { <VolatilitySkeleton /> }.into_any(),
        AssetVolatilityState::Ready(model) => view! { <VolatilityContent model /> }.into_any(),
        AssetVolatilityState::Unavailable { symbol } => view! { <VolatilityState symbol state=FeedbackState::unavailable() retry=None /> }.into_any(),
        AssetVolatilityState::RecoverableError { symbol } => { let retry=Callback::new(move |_| set_retried.set(true)); view! { <VolatilityState symbol state=FeedbackState::recoverable_error() retry=Some(retry) /> }.into_any() }
    }}}
}

#[component]
fn VolatilityContent(model: AssetVolatilityReadModel) -> impl IntoView {
    let selected_view = RwSignal::new(VolatilityView::Surface);
    let grid = model.grid.clone();
    let surface_grid = grid.clone();
    let heatmap_grid = grid.clone();
    let analytics_moneyness = grid.moneyness.clone();
    view! {
        <div class="xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
            <VolatilityHeader model=model.clone() />
            <main class="flex min-h-0 flex-1 flex-col gap-2 bg-canvas p-2 xl:overflow-hidden">
                <VolatilityFilters model=model.clone() />
                <div class="grid min-h-0 flex-1 gap-2 xl:grid-cols-[minmax(0,1fr)_22rem] xl:grid-rows-[minmax(25rem,1fr)_minmax(17rem,0.58fr)]">
                    <section class="flex min-h-0 flex-col border border-border bg-surface" aria-label="Volatility visualization">
                        <div class="panel-header gap-4"><h2 class="text-sm font-semibold">"Implied Volatility · Moneyness × Days to Expiry"</h2><div class="ml-auto flex rounded border border-border bg-canvas p-0.5" role="group" aria-label="Volatility visualization"><button type="button" class=move || if selected_view.get()==VolatilityView::Surface { "min-h-8 bg-state-selected px-3 text-xs text-interactive-text" } else { "min-h-8 px-3 text-xs text-text-secondary" } aria-pressed=move || selected_view.get()==VolatilityView::Surface on:click=move |_| selected_view.set(VolatilityView::Surface)>"Surface 3D"</button><button type="button" class=move || if selected_view.get()==VolatilityView::Heatmap { "min-h-8 bg-state-selected px-3 text-xs text-interactive-text" } else { "min-h-8 px-3 text-xs text-text-secondary" } aria-pressed=move || selected_view.get()==VolatilityView::Heatmap on:click=move |_| selected_view.set(VolatilityView::Heatmap)>"Heatmap"</button></div></div>
                        {move || if selected_view.get()==VolatilityView::Surface { view! { <VolatilitySurfaceChart grid=surface_grid.clone() /> }.into_any() } else { view! { <VolatilityHeatmap grid=heatmap_grid.clone() /> }.into_any() }}
                    </section>
                    <VolatilitySnapshot metrics=model.snapshot_metrics.clone() as_of=model.as_of.clone() />
                    <div class="min-h-0 xl:col-span-2"><VolatilityAnalytics moneyness=analytics_moneyness smiles=model.smiles.clone() term_structure=model.term_structure.clone() /></div>
                </div>
            </main>
        </div>
    }
}

#[component]
fn VolatilityHeader(model: AssetVolatilityReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    view! { <header class="shrink-0 border-b border-border bg-canvas px-4 pt-3 sm:px-6"><div class="flex min-h-[4.75rem] items-start justify-between gap-4 pb-2"><div><div class="flex flex-wrap items-baseline gap-3"><h1 class="numeric text-[1.75rem] font-black">{model.symbol.clone()}</h1><span class="text-sm font-medium">{model.name}</span><span class="text-xs text-text-secondary">"•"</span><span class="text-xs text-text-secondary">{model.venue}</span></div><div class="mt-2 flex items-center gap-3 numeric"><span class="text-[1.625rem] font-semibold">{model.price}</span><span class=format!("text-sm font-semibold {change_class}")>{model.percentage_change}</span><span class="mock-indicator">"Mock volatility"</span></div></div></div><AssetTabs ticker=model.symbol capabilities=model.capabilities /></header> }
}

#[component]
fn VolatilityFilters(model: AssetVolatilityReadModel) -> impl IntoView {
    view! { <section class="dense-scrollbar flex shrink-0 gap-3 overflow-x-auto border border-border bg-surface p-2" aria-label="Volatility filters">{[("Metric",model.metric),("Type",model.option_type),("Expirations",model.expiration_filter),("Normalization",model.normalization)].into_iter().map(|(label,value)| view! { <label class="block min-w-48 flex-1 text-[0.6875rem] text-text-secondary"><span class="mb-1 block">{label}</span><select class="min-h-10 w-full border border-border bg-canvas px-3 text-sm font-medium text-text-primary" aria-label=label><option>{value}</option></select></label> }).collect_view()}</section> }
}

#[component]
fn VolatilityState(
    symbol: String,
    state: FeedbackState,
    retry: Option<Callback<()>>,
) -> impl IntoView {
    view! { <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol /></header><div class="p-4 sm:p-6"><Panel title="Asset Volatility"><DataState state />{retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_| action.run(())>"Retry locally"</button></div> })}</Panel></div> }
}

#[component]
fn VolatilitySkeleton() -> impl IntoView {
    view! { <div aria-busy="true" aria-label="Loading asset volatility"><div class="h-32 animate-pulse border-b border-border bg-surface"></div><div class="grid gap-2 p-2 xl:grid-cols-[minmax(0,1fr)_22rem]"><div class="h-[32rem] animate-pulse border border-border bg-surface"></div><div class="h-[32rem] animate-pulse border border-border bg-surface"></div></div></div> }
}
