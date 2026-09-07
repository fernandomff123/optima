use crate::{
    application::{
        asset_options::AssetOptionsState,
        asset_simulation::{AssetSimulationReadModel, AssetSimulationState},
        read_models::FeedbackState,
    },
    composition::{asset_options_use_case, asset_simulation_use_case, strategy_simulation_port},
    driving_adapters::ui::{
        components::{
            AssetTabs, DataState, Panel, ScenarioSelection, SimulationLegPicker,
            SimulationMetricStrip, SimulationPosition, SimulationScenarioPanel,
        },
        echarts::{
            SimulationPayoffChart, backend_greeks_at_spot, backend_payoff_metrics,
            backend_pnl_at_spot,
        },
        simulation_draft::{
            DraftLeg, base_draft_legs, read_draft_legs, strategy_simulation_request,
            write_draft_legs,
        },
    },
    ports::{
        asset_options::OptionsScenario,
        asset_simulation::{
            SimulationScenario, StrategySimulationFailure, StrategySimulationResult,
        },
    },
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
    view! { {move || match state() {
        AssetSimulationState::Loading => view! { <SimulationSkeleton /> }.into_any(),
        AssetSimulationState::Ready(model) => view! { <SimulationContent model /> }.into_any(),
        AssetSimulationState::Unavailable { symbol } => view! { <SimulationState symbol state=FeedbackState::unavailable() retry=None /> }.into_any(),
        AssetSimulationState::RecoverableError { symbol } => { let retry = Callback::new(move |_| set_retried.set(true)); view! { <SimulationState symbol state=FeedbackState::recoverable_error() retry=Some(retry) /> }.into_any() }
    }} }
}

#[component]
fn SimulationContent(model: AssetSimulationReadModel) -> impl IntoView {
    let symbol = model.symbol.clone();
    let strategy_name = model.strategy_name.clone();
    let expiration_date = model.expiration_date.clone();
    let draft_rows = RwSignal::new(initial_draft(&model));
    let payoff_model = model.clone();
    let metric_model = model.clone();
    let scenario_preset = model.preset.clone();
    let scenario_controls = model.controls.clone();
    let base_selection = ScenarioSelection::from_controls(&scenario_controls);
    let current_implied_volatility = scenario_controls
        .iter()
        .find(|control| control.label == "Implied Volatility")
        .and_then(|control| numeric_value(&control.current))
        .unwrap_or(base_selection.implied_volatility);
    let scenario_selection = RwSignal::new(base_selection);
    let pricing = RwSignal::<Option<StrategySimulationResult>>::new(None);
    let pricing_error = RwSignal::<Option<String>>::new(None);
    let pricing_loading = RwSignal::new(false);
    let pricing_revision = RwSignal::new(0_u64);
    let strategy_editor_open = RwSignal::new(false);
    let editor_underlying = RwSignal::new(false);
    let editor_open_signal = Signal::derive(move || strategy_editor_open.get());
    let heatmap = model.heatmap.clone();
    let summary_heatmap = heatmap.clone();
    let current_spot = model.current_spot;
    let options_model =
        match asset_options_use_case().execute(&model.symbol, OptionsScenario::Normal) {
            AssetOptionsState::Ready(options) => Some(options),
            _ => None,
        };
    let open_option_editor = Callback::new(move |_| {
        editor_underlying.set(false);
        strategy_editor_open.set(true);
    });
    let open_underlying_editor = Callback::new(move |_| {
        editor_underlying.set(true);
        strategy_editor_open.set(true);
    });
    let close_strategy_editor = Callback::new(move |_| strategy_editor_open.set(false));
    let pricing_symbol = symbol.clone();
    let pricing_valuation_date = model.current_date.clone();
    let pricing_port = strategy_simulation_port();
    Effect::new(move |_| {
        let draft = draft_rows.get();
        let selected = scenario_selection.get();
        let volatility_shift = (selected.implied_volatility - current_implied_volatility) / 100.0;
        if draft.is_empty() {
            pricing.set(None);
            pricing_error.set(None);
            pricing_loading.set(false);
            return;
        }
        let request = match strategy_simulation_request(
            &pricing_symbol,
            &draft,
            &pricing_valuation_date,
            current_spot,
            selected.spot,
            current_implied_volatility / 100.0,
            volatility_shift,
            selected.time_days,
        ) {
            Ok(request) => request,
            Err(message) => {
                pricing.set(None);
                pricing_error.set(Some(message));
                pricing_loading.set(false);
                return;
            }
        };
        pricing_revision.update(|revision| *revision += 1);
        let revision = pricing_revision.get_untracked();
        pricing_loading.set(true);
        pricing_error.set(None);
        let pricing_port = pricing_port.clone();
        leptos::task::spawn_local(async move {
            let result = pricing_port.simulate(request).await;
            if pricing_revision.get_untracked() != revision {
                return;
            }
            pricing_loading.set(false);
            match result {
                Ok(result) => {
                    pricing.set(Some(result));
                    pricing_error.set(None);
                }
                Err(error) => {
                    pricing.set(None);
                    pricing_error.set(Some(pricing_failure_message(error)));
                }
            }
        });
    });

    view! {
        <div class="xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
            <SimulationHeader model=model />
            <main class=move || if strategy_editor_open.get() { "grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:grid-cols-[minmax(22rem,0.9fr)_minmax(38rem,1.7fr)_minmax(20rem,0.85fr)] xl:grid-rows-[minmax(22rem,1fr)_4.375rem_minmax(17rem,0.68fr)] xl:overflow-hidden" } else { "grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:grid-cols-[minmax(22rem,0.9fr)_minmax(38rem,1.7fr)_minmax(20rem,0.85fr)] xl:grid-rows-[minmax(22rem,1fr)_4.375rem_minmax(13rem,0.52fr)] xl:overflow-hidden" }>
                <div class=move || if strategy_editor_open.get() { "min-h-0 xl:row-span-2" } else { "min-h-0 xl:row-start-1 xl:mb-10" }><SimulationPosition symbol=symbol.clone() strategy_name rows=draft_rows editor_open=editor_open_signal on_add_option=open_option_editor on_add_underlying=open_underlying_editor on_done=close_strategy_editor /></div>
                <section class="flex min-h-0 min-w-0 flex-col overflow-hidden border border-border bg-surface xl:col-start-2" aria-label="Simulation result">
                    <div class="panel-header"><h2 class="text-sm font-semibold">"Result"</h2><div class="flex items-center gap-3"><span class="numeric text-[0.6875rem] text-text-secondary">{move || if draft_rows.get().is_empty() { "No position".to_owned() } else if pricing_loading.get() { "Calculating Black–Scholes…".to_owned() } else if let Some(error) = pricing_error.get() { error } else if let Some(result) = pricing.get() { let selected = scenario_selection.get(); let pnl = backend_pnl_at_spot(&result, selected.spot).map(|value| format!(" · P&L {value:+.0}")).unwrap_or_default(); format!("Spot {:.2} · IV {:.1}% · r 0.0% · q 0.0%{pnl}", selected.spot, selected.implied_volatility) } else { "Waiting for simulation".to_owned() }}</span><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">{move || if pricing.get().is_some() { "Black–Scholes" } else { "Simulation" }}</span></div></div>
                    <ResultTabs />
                    <SimulationPayoffChart model=payoff_model selection=scenario_selection draft_rows=draft_rows pricing=pricing.read_only() />
                </section>
                <div class="min-h-0 xl:col-start-3 xl:row-span-2"><SimulationScenarioPanel preset=scenario_preset controls=scenario_controls selection=scenario_selection /></div>
                <div class="xl:col-start-2 xl:row-start-2"><SimulationMetricStrip metrics=Signal::derive(move || backend_payoff_metrics(&metric_model, &draft_rows.get(), pricing.get().as_ref())) /></div>
                {move || if strategy_editor_open.get() {
                    view! { <div class="min-h-0 xl:col-span-3 xl:row-start-3">{options_model.clone().map(|options| view! { <SimulationLegPicker model=options draft_rows start_underlying=editor_underlying.get() on_close=close_strategy_editor /> }.into_any()).unwrap_or_else(|| view! { <section class="flex h-full items-center justify-center border border-border bg-surface text-sm text-text-secondary">"Option strikes are unavailable for this mock asset."</section> }.into_any())}</div> }.into_any()
                } else {
                    view! { <div class="min-h-0 xl:col-start-1 xl:row-start-2 xl:row-span-2 xl:-mt-10"><StrategyDetails expiration=expiration_date.clone() legs=draft_rows /></div><div class="min-h-0 xl:col-start-2 xl:row-start-3"><LiveSensitivity selection=scenario_selection legs=draft_rows pricing=pricing.read_only() /></div><div class="min-h-0 xl:col-start-3 xl:row-start-3"><ScenarioSummary current_spot current_implied_volatility heatmap=summary_heatmap.clone() selection=scenario_selection legs=draft_rows pricing=pricing.read_only() /></div> }.into_any()
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
        if leg.instrument.eq_ignore_ascii_case("STOCK") && leg.price.eq_ignore_ascii_case("Market")
        {
            leg.price = model.price.clone();
        }
    }
    write_draft_legs(&initial);
    initial
}

#[component]
fn StrategyDetails(expiration: String, legs: RwSignal<Vec<DraftLeg>>) -> impl IntoView {
    view! { <section class="flex h-full min-h-0 flex-col border border-border bg-surface"><div class="panel-header"><h3 class="pt-0.5 text-sm font-medium uppercase tracking-wide text-text-secondary">"Strategy Details"</h3></div>{move || if legs.get().is_empty() { view! { <p class="m-auto text-[0.8125rem] text-text-secondary">"No position"</p> }.into_any() } else { let expiration = expiration.clone(); view! { <dl class="simulation-detail-list"><div class="simulation-detail-row"><dt class="text-text-secondary">"Expiry"</dt><dd>{expiration}</dd></div><div class="simulation-detail-row"><dt class="text-text-secondary">"Contracts"</dt><dd class="numeric">{legs.get().iter().filter(|leg| !leg.instrument.eq_ignore_ascii_case("STOCK")).map(|leg| leg.quantity.unsigned_abs()).sum::<u32>()}</dd></div><div class="simulation-detail-row"><dt class="text-text-secondary">"Multiplier"</dt><dd class="numeric">"100"</dd></div></dl> }.into_any() }}</section> }
}

#[component]
fn LiveSensitivity(
    selection: RwSignal<ScenarioSelection>,
    legs: RwSignal<Vec<DraftLeg>>,
    pricing: ReadSignal<Option<StrategySimulationResult>>,
) -> impl IntoView {
    view! {
        <section class="flex h-full min-h-0 flex-col border border-border bg-surface">
            <div class="panel-header"><h3 class="pt-0.5 text-sm font-medium uppercase tracking-wide text-text-secondary">"Live Sensitivity"</h3></div>
            {move || if legs.get().is_empty() {
                view! { <p class="m-auto text-xs text-text-secondary">"No position"</p> }.into_any()
            } else {
                let selected = selection.get();
                let values = pricing.get().as_ref().and_then(|result| backend_greeks_at_spot(result, selected.spot));
                let items = values.map(|greeks| vec![
                    ("Δ", greeks.delta),
                    ("Γ", greeks.gamma),
                    ("Θ", greeks.theta),
                    ("Vega", greeks.vega),
                ]).unwrap_or_else(|| vec![("Δ", f64::NAN), ("Γ", f64::NAN), ("Θ", f64::NAN), ("Vega", f64::NAN)]);
                view! {
                    <div class="grid min-h-0 flex-1 grid-cols-5 items-start divide-x divide-border px-3 pt-5">
                        {items.into_iter().map(|(label, value)| view! {
                            <div class="px-3 text-center">
                                <p class="text-xs leading-none text-text-secondary">{label}</p>
                                <p class=greek_value_class(value)>{format_greek(value)}</p>
                            </div>
                        }).collect_view()}
                        <div class="px-3 text-center text-xs text-text-secondary"><p class="leading-none">"Full analysis in"</p><p class="mt-2.5 leading-none text-interactive-text">"Greeks tab"</p></div>
                    </div>
                    <div class="border-t border-border px-4 py-3 text-center text-xs text-text-secondary numeric">{format!("Scenario: Spot {:.2}  ·  IV {:.1}%  ·  +{:.0} days", selected.spot, selected.implied_volatility, selected.time_days)}</div>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn ScenarioSummary(
    current_spot: f64,
    current_implied_volatility: f64,
    heatmap: crate::application::asset_simulation::PnlHeatmap,
    selection: RwSignal<ScenarioSelection>,
    legs: RwSignal<Vec<DraftLeg>>,
    pricing: ReadSignal<Option<StrategySimulationResult>>,
) -> impl IntoView {
    let _ = heatmap;
    view! { <section class="flex h-full min-h-0 flex-col border border-border bg-surface"><div class="panel-header"><h3 class="pt-0.5 text-sm font-medium uppercase tracking-wide text-text-secondary">"Scenario Summary"</h3></div><dl class="simulation-detail-list"><div class="simulation-detail-row"><dt class="text-text-secondary">"Spot Change"</dt><dd class="numeric font-semibold text-finance-positive">{move || format!("{:+.1}%", (selection.get().spot / current_spot - 1.0) * 100.0)}</dd></div><div class="simulation-detail-row"><dt class="text-text-secondary">"IV Change"</dt><dd class="numeric font-semibold text-finance-positive">{move || format!("{:+.1} pts", selection.get().implied_volatility - current_implied_volatility)}</dd></div><div class="simulation-detail-row"><dt class="text-text-secondary">"Time Change"</dt><dd class="numeric font-semibold text-finance-positive">{move || format!("+{:.0} days", selection.get().time_days)}</dd></div><div class="simulation-detail-row"><dt class="font-medium text-text-primary">"Estimated P&L"</dt><dd class="numeric text-base font-semibold text-finance-positive">{move || if legs.get().is_empty() { "—".to_owned() } else { pricing.get().as_ref().and_then(|result| backend_pnl_at_spot(result, selection.get().spot)).map(|value| format!("{value:+.0}")).unwrap_or_else(|| "—".to_owned()) }}</dd></div></dl></section> }
}

fn pricing_failure_message(error: StrategySimulationFailure) -> String {
    match error {
        StrategySimulationFailure::Unsupported(message) => message,
        StrategySimulationFailure::Transport(message) => format!("Backend unavailable · {message}"),
        StrategySimulationFailure::InvalidResponse(message) => {
            format!("Invalid backend response · {message}")
        }
    }
}

fn greek_value_class(value: f64) -> &'static str {
    if value.is_nan() {
        "mt-2.5 numeric text-lg font-semibold leading-none text-text-secondary"
    } else if value < 0.0 {
        "mt-2.5 numeric text-lg font-semibold leading-none text-negative-text"
    } else {
        "mt-2.5 numeric text-lg font-semibold leading-none text-finance-positive"
    }
}

fn format_greek(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.4}")
    } else {
        "—".to_owned()
    }
}
fn numeric_value(value: &str) -> Option<f64> {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_digit() || matches!(character, '.' | '-'))
        .collect::<String>();
    normalized.parse().ok()
}

#[component]
fn ResultTabs() -> impl IntoView {
    view! { <div class="dense-scrollbar flex h-11 shrink-0 overflow-x-auto border-b border-border text-xs"><button type="button" class="border-b-2 border-interactive-text px-5 font-semibold text-interactive-text" aria-pressed=true>"Payoff"</button>{["P&L by Date", "Greeks", "P&L Heatmap", "Monte Carlo"].into_iter().map(|label| view! { <button type="button" class="cursor-not-allowed border-b-2 border-transparent px-5 text-text-secondary opacity-60" disabled>{label}</button> }).collect_view()}</div> }
}

#[component]
fn SimulationHeader(model: AssetSimulationReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    view! { <header class="shrink-0 border-b border-border bg-canvas px-4 pt-3 sm:px-6"><div class="flex min-h-[4.75rem] items-start justify-between gap-4 pb-2"><div><div class="flex flex-wrap items-baseline gap-3"><h1 class="numeric text-[1.75rem] font-black">{model.symbol.clone()}</h1><span class="text-sm font-medium">{model.name}</span><span class="text-xs text-text-secondary">"•"</span><span class="text-xs text-text-secondary">{model.venue}</span></div><div class="mt-2 flex items-center gap-3 numeric"><span class="text-[1.625rem] font-semibold">{model.price}</span><span class=format!("text-sm font-semibold {change_class}")>{model.percentage_change}</span><span class="mock-indicator">"Mock simulation"</span></div></div></div><AssetTabs ticker=model.symbol capabilities=model.capabilities /></header> }
}

#[component]
fn SimulationState(
    symbol: String,
    state: FeedbackState,
    retry: Option<Callback<()>>,
) -> impl IntoView {
    view! { <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol /></header><div class="p-4 sm:p-6"><Panel title="Asset Simulation"><DataState state />{retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_| action.run(())>"Retry locally"</button></div> })}</Panel></div> }
}

#[component]
fn SimulationSkeleton() -> impl IntoView {
    view! { <div aria-busy="true" aria-label="Loading asset simulation"><div class="h-32 animate-pulse border-b border-border bg-surface"></div><div class="grid gap-2 p-2 xl:grid-cols-3"><div class="h-[30rem] animate-pulse border border-border bg-surface"></div><div class="h-[30rem] animate-pulse border border-border bg-surface"></div><div class="h-[30rem] animate-pulse border border-border bg-surface"></div></div></div> }
}
