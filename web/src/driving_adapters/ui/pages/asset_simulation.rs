use crate::{
    application::{
        asset_options::AssetOptionsState,
        asset_simulation::{AssetSimulationReadModel, AssetSimulationState},
        read_models::FeedbackState,
    },
    composition::{asset_options_use_case, asset_simulation_use_case},
    driving_adapters::ui::{
        components::{
            AssetTabs, DataState, Panel, ScenarioSelection, SimulationGreeks,
            SimulationLegPicker, SimulationMetricStrip, SimulationPnlHeatmap,
            SimulationPosition, SimulationScenarioPanel,
        },
        echarts::{SimulationPayoffChart, SimulationPnlByDateChart, SimulationTimeDecayChart},
        simulation_draft::{DraftLeg, base_draft_legs, read_draft_legs, write_draft_legs},
    },
    ports::{
        asset_options::OptionsScenario,
        asset_simulation::SimulationScenario,
    },
};
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResultView {
    Payoff,
    PnlByDate,
    Greeks,
    PnlHeatmap,
    MonteCarlo,
}

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
    let strategy_name = model.strategy_name.clone();
    let draft_rows = RwSignal::new(initial_draft(&model));
    let result_model = model.clone();
    let lower_result_model = model.clone();
    let scenario_preset = model.preset.clone();
    let scenario_controls = model.controls.clone();
    let scenario_selection = RwSignal::new(ScenarioSelection::from_controls(&scenario_controls));
    let result_view = RwSignal::new(ResultView::Payoff);
    let strategy_editor_open = RwSignal::new(false);
    let metrics = model.metrics.clone();
    let heatmap = model.heatmap.clone();
    let result_heatmap = heatmap.clone();
    let summary_heatmap = result_heatmap.clone();
    let greeks = model.greeks.clone();
    let probability_low = model.probability_low.clone();
    let probability_high = model.probability_high.clone();
    let options_model = match asset_options_use_case().execute(&model.symbol, OptionsScenario::Normal) {
        AssetOptionsState::Ready(options) => Some(options),
        _ => None,
    };
    let open_strategy_editor = Callback::new(move |_| {
        result_view.set(ResultView::Payoff);
        strategy_editor_open.set(true);
    });
    let close_strategy_editor = Callback::new(move |_| strategy_editor_open.set(false));

    view! {
        <div class="xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
            <SimulationHeader model=model />
            <main class=move || if strategy_editor_open.get() || result_view.get() == ResultView::PnlByDate { "grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:grid-cols-[25.5rem_minmax(0,1fr)_20rem] xl:grid-rows-[minmax(20rem,1fr)_4.375rem_minmax(14rem,0.72fr)] xl:overflow-hidden" } else { "grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:grid-cols-[25.5rem_minmax(0,1fr)_20rem] xl:grid-rows-[minmax(20rem,1fr)_4.375rem] xl:overflow-hidden" }>
                <div class="min-h-0"><SimulationPosition strategy_name rows=draft_rows on_add_leg=open_strategy_editor /></div>
                <section class="flex min-h-0 min-w-0 flex-col overflow-hidden border border-border bg-surface xl:col-start-2" aria-label="Simulation result">
                    <div class="panel-header"><h2 class="text-sm font-semibold">"Result"</h2><div class="flex items-center gap-3"><span class="numeric text-[0.6875rem] text-text-secondary">{move || { let selected = scenario_selection.get(); let pnl = selected_fixture_pnl(&result_heatmap, selected); format!("Spot {:.2} · IV {:.1}% · +{:.0}d · Fixture P&L ${pnl:.0}", selected.spot, selected.implied_volatility, selected.time_days) }}</span><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Mock snapshot"</span></div></div>
                    <ResultTabs selected=result_view strategy_editor_open />
                    <div class="min-h-0 flex-1">
                        {move || match result_view.get() {
                            ResultView::Payoff => view! {
                                <div class="flex h-full min-h-0 flex-col">
                                    <SimulationPayoffChart model=result_model.clone() selection=scenario_selection />
                                    <div class="flex shrink-0 items-center gap-3 border-t border-border px-6 py-2 text-[0.6875rem] text-text-secondary numeric"><span>{probability_low.clone()}</span><span class="h-px flex-1 bg-border"></span><span>"68% Probability"</span><span class="h-px flex-1 bg-border"></span><span>{probability_high.clone()}</span></div>
                                </div>
                            }.into_any(),
                            ResultView::PnlByDate => view! { <SimulationPnlByDateChart model=result_model.clone() selection=scenario_selection /> }.into_any(),
                            ResultView::Greeks => view! { <SimulationGreeks greeks=greeks.clone() /> }.into_any(),
                            ResultView::PnlHeatmap => view! { <SimulationPnlHeatmap heatmap=heatmap.clone() selection=scenario_selection /> }.into_any(),
                            ResultView::MonteCarlo => view! { <SimulationMonteCarlo /> }.into_any(),
                        }}
                    </div>
                </section>
                <div class="min-h-0 xl:col-start-3 xl:row-span-2"><SimulationScenarioPanel preset=scenario_preset controls=scenario_controls selection=scenario_selection /></div>
                <div class="xl:col-span-2 xl:row-start-2"><SimulationMetricStrip metrics /></div>
                {move || if strategy_editor_open.get() {
                    Some(view! { <div class="min-h-0 xl:col-span-3 xl:row-start-3">{options_model.clone().map(|options| view! { <SimulationLegPicker model=options draft_rows on_close=close_strategy_editor /> }.into_any()).unwrap_or_else(|| view! { <section class="flex h-full items-center justify-center border border-border bg-surface text-sm text-text-secondary">"Option strikes are unavailable for this mock asset."</section> }.into_any())}</div> }.into_any())
                } else if result_view.get() == ResultView::PnlByDate {
                    Some(view! {
                        <div class="min-h-0 overflow-hidden border border-border bg-surface xl:col-span-2 xl:row-start-3"><SimulationTimeDecayChart model=lower_result_model.clone() selection=scenario_selection /></div>
                        <div class="min-h-0 xl:col-start-3 xl:row-start-3"><SelectedScenarioSummary heatmap=summary_heatmap.clone() selection=scenario_selection /></div>
                    }.into_any())
                } else {
                    None
                }}
            </main>
        </div>
    }
}

fn initial_draft(model: &AssetSimulationReadModel) -> Vec<DraftLeg> {
    let base = base_draft_legs(&model.legs);
    let stored = read_draft_legs();
    let mut initial = if stored.is_empty() {
        base.clone()
    } else {
        stored
            .into_iter()
            .map(|stored_leg| {
                base.iter()
                    .find(|base_leg| base_leg.key == stored_leg.key)
                    .cloned()
                    .unwrap_or(stored_leg)
            })
            .collect()
    };
    for leg in &mut initial {
        if leg.instrument.eq_ignore_ascii_case("STOCK") && leg.price.eq_ignore_ascii_case("Market") {
            leg.price = model.price.clone();
        }
    }
    write_draft_legs(&initial);
    initial
}

#[component]
fn ResultTabs(
    selected: RwSignal<ResultView>,
    strategy_editor_open: RwSignal<bool>,
) -> impl IntoView {
    let tabs = [
        (ResultView::Payoff, "Payoff"),
        (ResultView::PnlByDate, "P&L by Date"),
        (ResultView::Greeks, "Greeks"),
        (ResultView::PnlHeatmap, "P&L Heatmap"),
        (ResultView::MonteCarlo, "Monte Carlo"),
    ];
    view! {
        <div class="dense-scrollbar flex h-11 shrink-0 overflow-x-auto border-b border-border text-xs" role="tablist" aria-label="Simulation result views">
            {tabs.into_iter().map(|(tab, label)| view! { <button type="button" role="tab" class=move || result_tab_class(selected.get() == tab) aria-selected=move || selected.get() == tab on:click=move |_| { strategy_editor_open.set(false); selected.set(tab); }>{label}</button> }).collect_view()}
        </div>
    }
}

fn result_tab_class(selected: bool) -> &'static str {
    if selected {
        "border-b-2 border-interactive-text px-5 font-semibold text-interactive-text"
    } else {
        "border-b-2 border-transparent px-5 text-text-secondary hover:text-text-primary"
    }
}

#[component]
fn SelectedScenarioSummary(
    heatmap: crate::application::asset_simulation::PnlHeatmap,
    selection: RwSignal<ScenarioSelection>,
) -> impl IntoView {
    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface" aria-label="Selected scenario summary">
            <div class="panel-header"><h3 class="text-sm font-semibold">"Selected Scenario Summary"</h3></div>
            <dl class="space-y-3 p-4 text-xs numeric">
                <div class="flex justify-between gap-3"><dt class="text-text-secondary">"Selected scenario"</dt><dd class="text-text-primary">{move || if selection.get().time_days == 0.0 { "Today".to_owned() } else { format!("+{:.0} days", selection.get().time_days) }}</dd></div>
                <div class="flex justify-between gap-3"><dt class="text-text-secondary">"Spot price"</dt><dd class="text-text-primary">{move || format!("{:.2}", selection.get().spot)}</dd></div>
                <div class="flex justify-between gap-3"><dt class="text-text-secondary">"Implied volatility"</dt><dd class="text-text-primary">{move || format!("{:.1}%", selection.get().implied_volatility)}</dd></div>
                <div class="flex justify-between gap-3 border-t border-border pt-3"><dt class="text-text-secondary">"Fixture P&L"</dt><dd class="font-semibold text-interactive-text">{move || format!("${:.0}", selected_fixture_pnl(&heatmap, selection.get()))}</dd></div>
            </dl>
        </section>
    }
}

#[component]
fn SimulationMonteCarlo() -> impl IntoView {
    view! { <section class="flex h-full items-center justify-center bg-surface p-8 text-center"><div><p class="text-sm font-semibold text-text-primary">"Monte Carlo"</p><p class="mt-2 max-w-md text-xs leading-relaxed text-text-secondary">"This tab is ready for a dedicated probability-distribution contract. No random market values are invented by the UI fixture."</p></div></section> }
}

fn selected_fixture_pnl(heatmap: &crate::application::asset_simulation::PnlHeatmap, selection: ScenarioSelection) -> f64 {
    let row = nearest(&heatmap.implied_volatilities, selection.implied_volatility);
    let column = nearest(&heatmap.spot_prices, selection.spot);
    heatmap.values.get(row).and_then(|values| values.get(column)).copied().unwrap_or_default()
}

fn nearest(values: &[f64], target: f64) -> usize {
    values.iter().enumerate().min_by(|(_, left), (_, right)| (**left - target).abs().total_cmp(&(**right - target).abs())).map(|(index, _)| index).unwrap_or_default()
}

#[component]
fn SimulationHeader(model: AssetSimulationReadModel) -> impl IntoView {
    let change_class = if model.change_positive { "text-finance-positive" } else { "text-negative-text" };
    view! { <header class="shrink-0 border-b border-border bg-canvas px-4 pt-3 sm:px-6"><div class="flex min-h-[4.75rem] items-start justify-between gap-4 pb-2"><div><div class="flex flex-wrap items-baseline gap-3"><h1 class="numeric text-[1.75rem] font-black">{model.symbol.clone()}</h1><span class="text-sm font-medium">{model.name}</span><span class="text-xs text-text-secondary">"•"</span><span class="text-xs text-text-secondary">{model.venue}</span></div><div class="mt-2 flex items-center gap-3 numeric"><span class="text-[1.625rem] font-semibold">{model.price}</span><span class=format!("text-sm font-semibold {change_class}")>{model.percentage_change}</span><span class="mock-indicator">"Mock simulation"</span></div></div></div><AssetTabs ticker=model.symbol capabilities=model.capabilities /></header> }
}

#[component]
fn SimulationState(symbol: String, state: FeedbackState, retry: Option<Callback<()>>) -> impl IntoView {
    view! { <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol /></header><div class="p-4 sm:p-6"><Panel title="Asset Simulation"><DataState state />{retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_| action.run(())>"Retry locally"</button></div> })}</Panel></div> }
}

#[component]
fn SimulationSkeleton() -> impl IntoView {
    view! { <div aria-busy="true" aria-label="Loading asset simulation"><div class="h-32 animate-pulse border-b border-border bg-surface"></div><div class="grid gap-2 p-2 xl:grid-cols-[25.5rem_minmax(0,1fr)_20rem]"><div class="h-[30rem] animate-pulse border border-border bg-surface"></div><div class="h-[30rem] animate-pulse border border-border bg-surface"></div><div class="h-[30rem] animate-pulse border border-border bg-surface"></div></div></div> }
}
