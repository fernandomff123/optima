use crate::{
    application::{
        asset_options::{AssetOptionsReadModel, AssetOptionsState},
        read_models::FeedbackState,
    },
    composition::asset_options_use_case,
    driving_adapters::ui::{
        components::{
            AssetTabs, DataState, OptionsAssetHeader, OptionsChain, OptionsContractPanel,
            OptionsToolbar, Panel,
        },
        plotly::OptionsSmileChart,
    },
    ports::asset_options::OptionsScenario,
};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

#[component]
pub fn AssetOptionsPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let (retried, set_retried) = signal(false);
    let state = move || {
        let ticker = params
            .read()
            .get("ticker")
            .unwrap_or_else(|| "AAPL".to_owned());
        let requested = OptionsScenario::from_query(query.read().get("scenario").as_deref());
        let scenario = if retried.get() && requested == OptionsScenario::RecoverableError {
            OptionsScenario::Normal
        } else {
            requested
        };
        asset_options_use_case().execute(&ticker, scenario)
    };
    view! {
        {move || match state() {
            AssetOptionsState::Loading => view! { <OptionsSkeleton /> }.into_any(),
            AssetOptionsState::Ready(model) => view! { <OptionsContent model /> }.into_any(),
            AssetOptionsState::Unavailable { symbol } => view! { <OptionsState symbol state=FeedbackState::unavailable() retry=None /> }.into_any(),
            AssetOptionsState::RecoverableError { symbol } => {
                let retry = Callback::new(move |_| set_retried.set(true));
                view! { <OptionsState symbol state=FeedbackState::recoverable_error() retry=Some(retry) /> }.into_any()
            }
        }}
    }
}

#[component]
fn OptionsContent(model: AssetOptionsReadModel) -> impl IntoView {
    let chain = model.chain.clone();
    let smile = model.smile.clone();
    let contract = model.contract.clone();
    view! {
        <div class="xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
            <div class="xl:shrink-0"><OptionsAssetHeader model=model.clone() /></div>
            <div class="xl:shrink-0"><OptionsToolbar model=model /></div>
            <main class="grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:overflow-hidden xl:grid-cols-[minmax(0,1fr)_18rem]">
                <div class="grid min-h-0 gap-[7px] xl:grid-rows-[minmax(0,1.35fr)_minmax(0,1fr)]">
                    <Panel title="AAPL Options Chain · Mock" compact=true><OptionsChain rows=chain /></Panel>
                    <Panel title="Options Analytics · 17 May 2025" compact=true><OptionsSmileChart smile /></Panel>
                </div>
                <OptionsContractPanel contract />
            </main>
        </div>
    }
}

#[component]
fn OptionsState(
    symbol: String,
    state: FeedbackState,
    retry: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol /></header>
        <div class="p-4 sm:p-6"><Panel title="Asset Options"><DataState state />{retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_| action.run(())>"Retry locally"</button></div> })}</Panel></div>
    }
}

#[component]
fn OptionsSkeleton() -> impl IntoView {
    view! { <div aria-busy="true" aria-label="Loading asset options"><div class="h-32 animate-pulse border-b border-border bg-surface"></div><div class="grid gap-2 p-2 xl:grid-cols-[minmax(0,1fr)_18rem]"><div class="space-y-2"><div class="h-[28rem] animate-pulse border border-border bg-surface"></div><div class="h-80 animate-pulse border border-border bg-surface"></div></div><div class="h-[40rem] animate-pulse border border-border bg-surface"></div></div></div> }
}
