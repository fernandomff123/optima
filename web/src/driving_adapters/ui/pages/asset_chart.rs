use crate::{
    application::{
        asset_chart::{AssetChartReadModel, AssetChartState},
        read_models::FeedbackState,
    },
    composition::asset_chart_use_case,
    driving_adapters::ui::{
        components::{AssetTabs, DataState, Panel},
        echarts::AssetCandlestickChart,
    },
    ports::asset_chart::ChartScenario,
};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

#[component]
pub fn AssetChartPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let (retried, set_retried) = signal(false);
    let state = move || {
        let ticker = params.read().get("ticker").unwrap_or_else(|| "AAPL".into());
        let requested = ChartScenario::from_query(query.read().get("scenario").as_deref());
        let scenario = if retried.get() && requested == ChartScenario::RecoverableError {
            ChartScenario::Normal
        } else {
            requested
        };
        asset_chart_use_case().execute(&ticker, scenario)
    };
    view! { {move || match state() {
        AssetChartState::Loading => view!{<ChartSkeleton/>}.into_any(),
        AssetChartState::Ready(model) => view!{<ChartContent model/>}.into_any(),
        AssetChartState::Unavailable{symbol} => view!{<ChartFeedback symbol state=FeedbackState::unavailable() retry=None/>}.into_any(),
        AssetChartState::RecoverableError{symbol} => { let retry=Callback::new(move |_|set_retried.set(true)); view!{<ChartFeedback symbol state=FeedbackState::recoverable_error() retry=Some(retry)/>}.into_any() }
    }} }
}

#[component]
fn ChartContent(model: AssetChartReadModel) -> impl IntoView {
    let candles = model.candles.clone();
    view! { <div class="xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
        <header class="shrink-0 border-b border-border bg-canvas px-4 pt-4 sm:px-6">
            <div class="flex min-h-24 items-start justify-between pb-3"><div><div class="flex flex-wrap items-baseline gap-x-4 gap-y-1"><h1 class="numeric text-[2.125rem] font-black leading-none">{model.symbol.clone()}</h1><span class="font-medium">{model.name}</span><span class="text-text-muted-readable">"•"</span><span class="text-sm text-text-secondary">{model.venue}</span></div><div class="mt-3 flex items-center gap-5 numeric"><span class="text-[1.75rem] font-semibold">{model.price}</span><span class="font-semibold text-finance-positive">{model.absolute_change} " (" {model.percentage_change} ")"</span><span class="h-5 border-l border-border"></span><span class="text-xs font-semibold text-finance-positive">{model.market_status}</span></div></div><span class="mock-indicator mt-2">"Mock data"</span></div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities/>
        </header>
        <div class="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-border bg-surface px-3 py-2 text-xs"><button class="border-b-2 border-interactive-text px-3 py-2 font-semibold">"1D"</button>{["5D","1M","3M","YTD","1Y","5Y"].into_iter().map(|label|view!{<button disabled class="cursor-not-allowed px-3 py-2 text-text-secondary">{label}</button>}).collect_view()}<span class="mx-2 h-6 border-l border-border"></span><span class="px-2 text-text-secondary">"Daily candles"</span><span class="rounded border border-border bg-surface-elevated px-2 py-1 text-text-muted-readable">"Deterministic fixture"</span></div>
        <main class="min-h-[34rem] flex-1 bg-canvas p-[7px] xl:min-h-0 xl:overflow-hidden"><Panel title="AAPL · 1D · NASDAQ — Price & Volume" compact=true><div class="h-full min-h-[30rem] xl:min-h-0"><AssetCandlestickChart candles/></div></Panel></main>
    </div> }
}

#[component]
fn ChartFeedback(
    symbol: String,
    state: FeedbackState,
    retry: Option<Callback<()>>,
) -> impl IntoView {
    view! {<header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol/></header><div class="p-4 sm:p-6"><Panel title="Asset Chart"><DataState state/>{retry.map(|action|view!{<div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_|action.run(())>"Retry locally"</button></div>})}</Panel></div>}
}
#[component]
fn ChartSkeleton() -> impl IntoView {
    view! {<div aria-busy="true" aria-label="Loading asset chart"><div class="h-36 animate-pulse border-b border-border bg-surface"></div><div class="m-2 h-[calc(100dvh-13rem)] min-h-[30rem] animate-pulse border border-border bg-surface"></div></div>}
}
