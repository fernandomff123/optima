use crate::{
    application::{
        asset_simulation::{AssetSimulationReadModel, AssetSimulationState},
        read_models::FeedbackState,
    },
    composition::asset_simulation_use_case,
    driving_adapters::ui::{
        components::{
            AssetTabs, DataState, Panel, SimulationGreeks, SimulationMetricStrip,
            SimulationPnlHeatmap, SimulationPosition, SimulationScenarioPanel,
        },
        echarts::SimulationPayoffChart,
    },
    ports::asset_simulation::SimulationScenario,
};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

#[component]
pub fn AssetSimulationPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let (retried, set_retried) = signal(false);
    let state = move || {
        let ticker = params
            .read()
            .get("ticker")
            .unwrap_or_else(|| "AAPL".to_owned());
        let requested = SimulationScenario::from_query(query.read().get("scenario").as_deref());
        let scenario = if retried.get() && requested == SimulationScenario::RecoverableError {
            SimulationScenario::Normal
        } else {
            requested
        };
        asset_simulation_use_case().execute(&ticker, scenario)
    };
    view! {
        {move || match state() {
            AssetSimulationState::Loading => view! { <SimulationSkeleton /> }.into_any(),
            AssetSimulationState::Ready(model) => view! { <SimulationContent model /> }.into_any(),
            AssetSimulationState::Unavailable { symbol } => view! { <SimulationState symbol state=FeedbackState::unavailable() retry=None /> }.into_any(),
            AssetSimulationState::RecoverableError { symbol } => {
                let retry = Callback::new(move |_| set_retried.set(true));
                view! { <SimulationState symbol state=FeedbackState::recoverable_error() retry=Some(retry) /> }.into_any()
            }
        }}
    }
}

#[component]
fn SimulationContent(model: AssetSimulationReadModel) -> impl IntoView {
    let position_strategy = model.strategy_name.clone();
    let position_legs = model.legs.clone();
    let payoff_model = model.clone();
    let scenario_preset = model.preset.clone();
    let scenario_controls = model.controls.clone();
    let metrics = model.metrics.clone();
    let heatmap = model.heatmap.clone();
    let greeks = model.greeks.clone();
    let probability_low = model.probability_low.clone();
    let probability_high = model.probability_high.clone();
    view! {
        <div class="xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
            <SimulationHeader model=model />
            <main class="grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:grid-cols-[18rem_minmax(0,1fr)_20rem] xl:grid-rows-[minmax(25rem,1fr)_4.375rem_minmax(15rem,0.72fr)] xl:overflow-hidden">
                <SimulationPosition strategy_name=position_strategy legs=position_legs />
                <section class="flex min-h-0 flex-col border border-border bg-surface xl:col-start-2" aria-label="Simulation result">
                    <div class="panel-header"><h2 class="text-sm font-semibold">"Result"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Mock snapshot"</span></div>
                    <ResultTabs />
                    <SimulationPayoffChart model=payoff_model />
                    <div class="flex shrink-0 items-center gap-3 border-t border-border px-6 py-2 text-[0.6875rem] text-text-secondary numeric"><span>{probability_low}</span><span class="h-px flex-1 bg-border"></span><span>"68% Probability"</span><span class="h-px flex-1 bg-border"></span><span>{probability_high}</span></div>
                </section>
                <div class="min-h-0 xl:col-start-3 xl:row-span-2"><SimulationScenarioPanel preset=scenario_preset controls=scenario_controls /></div>
                <div class="xl:col-span-2 xl:row-start-2"><SimulationMetricStrip metrics /></div>
                <div class="min-h-0 xl:col-span-2 xl:row-start-3"><SimulationPnlHeatmap heatmap /></div>
                <div class="min-h-0 xl:col-start-3 xl:row-start-3"><SimulationGreeks greeks /></div>
            </main>
        </div>
    }
}

#[component]
fn SimulationHeader(model: AssetSimulationReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    view! {
        <header class="shrink-0 border-b border-border bg-canvas px-4 pt-3 sm:px-6">
            <div class="flex min-h-[4.75rem] items-start justify-between gap-4 pb-2">
                <div><div class="flex flex-wrap items-baseline gap-3"><h1 class="numeric text-[1.75rem] font-black">{model.symbol.clone()}</h1><span class="text-sm font-medium">{model.name}</span><span class="text-xs text-text-secondary">"•"</span><span class="text-xs text-text-secondary">{model.venue}</span></div><div class="mt-2 flex items-center gap-3 numeric"><span class="text-[1.625rem] font-semibold">{model.price}</span><span class=format!("text-sm font-semibold {change_class}")>{model.percentage_change}</span><span class="mock-indicator">"Mock simulation"</span></div></div>
            </div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities />
        </header>
    }
}

#[component]
fn ResultTabs() -> impl IntoView {
    view! {
        <div class="dense-scrollbar flex h-11 shrink-0 overflow-x-auto border-b border-border text-xs">
            <button type="button" class="border-b-2 border-interactive-text px-5 font-semibold text-interactive-text" aria-pressed=true>"Payoff"</button>
            {[
                "P&L by Date", "Greeks", "P&L Heatmap", "Monte Carlo"
            ].into_iter().map(|label| view! { <button type="button" class="cursor-not-allowed border-b-2 border-transparent px-5 text-text-secondary opacity-60" disabled title="Requires a dedicated simulation result contract">{label}</button> }).collect_view()}
        </div>
    }
}

#[component]
fn SimulationState(
    symbol: String,
    state: FeedbackState,
    retry: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol /></header>
        <div class="p-4 sm:p-6"><Panel title="Asset Simulation"><DataState state />{retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_| action.run(())>"Retry locally"</button></div> })}</Panel></div>
    }
}

#[component]
fn SimulationSkeleton() -> impl IntoView {
    view! { <div aria-busy="true" aria-label="Loading asset simulation"><div class="h-32 animate-pulse border-b border-border bg-surface"></div><div class="grid gap-2 p-2 xl:grid-cols-[18rem_minmax(0,1fr)_20rem]"><div class="h-[30rem] animate-pulse border border-border bg-surface"></div><div class="h-[30rem] animate-pulse border border-border bg-surface"></div><div class="h-[30rem] animate-pulse border border-border bg-surface"></div></div></div> }
}
