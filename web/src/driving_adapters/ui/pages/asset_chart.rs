use crate::{
    application::{
        asset_chart::{AssetChartReadModel, AssetChartState},
        read_models::FeedbackState,
    },
    composition::asset_chart_use_case,
    driving_adapters::ui::{
        components::{AssetTabs, ChartIndicatorCatalog, DataState, Panel},
        echarts::{AssetChartCanvas, ChartVisibility},
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
        let ticker = params
            .read()
            .get("ticker")
            .unwrap_or_else(|| "AAPL".to_owned());
        let requested = ChartScenario::from_query(query.read().get("scenario").as_deref());
        let scenario = if retried.get() && requested == ChartScenario::RecoverableError {
            ChartScenario::Normal
        } else {
            requested
        };
        asset_chart_use_case().execute(&ticker, scenario)
    };
    view! {
        {move || match state() {
            AssetChartState::Loading => view! { <ChartSkeleton /> }.into_any(),
            AssetChartState::Ready(model) => view! { <ChartContent model /> }.into_any(),
            AssetChartState::Unavailable { symbol } => view! {
                <ChartState symbol state=FeedbackState::unavailable() retry=None />
            }.into_any(),
            AssetChartState::RecoverableError { symbol } => {
                let retry = Callback::new(move |_| set_retried.set(true));
                view! { <ChartState symbol state=FeedbackState::recoverable_error() retry=Some(retry) /> }.into_any()
            }
        }}
    }
}

#[component]
fn ChartContent(model: AssetChartReadModel) -> impl IntoView {
    let (ma20, set_ma20) = signal(true);
    let (ma50, set_ma50) = signal(true);
    let (ma200, set_ma200) = signal(true);
    let (bollinger, set_bollinger) = signal(false);
    let (rsi, set_rsi) = signal(true);
    let (macd, set_macd) = signal(true);
    let (catalog_open, set_catalog_open) = signal(false);
    let visibility = Signal::derive(move || ChartVisibility {
        ma20: ma20.get(),
        ma50: ma50.get(),
        ma200: ma200.get(),
        bollinger: bollinger.get(),
        rsi: rsi.get(),
        macd: macd.get(),
    });
    let canvas_model = model.clone();
    let sidebar_model = model.clone();
    view! {
        <div class="relative xl:flex xl:h-[calc(100dvh-3.5rem)] xl:min-h-0 xl:flex-col xl:overflow-hidden">
            <div class="xl:shrink-0"><ChartHeader model=model.clone() /></div>
            <div class="xl:shrink-0">
                <ChartToolbar catalog_open on_indicators=Callback::new(move |_| set_catalog_open.update(|open| *open = !*open)) on_reset=Callback::new(move |_| {
                    set_ma20.set(true); set_ma50.set(true); set_ma200.set(true);
                    set_bollinger.set(false); set_rsi.set(true); set_macd.set(true);
                    set_catalog_open.set(false);
                }) />
            </div>
            {move || catalog_open.get().then(|| view! {
                <ChartIndicatorCatalog
                    ma20 set_ma20 ma50 set_ma50 ma200 set_ma200 bollinger set_bollinger
                    rsi set_rsi macd set_macd
                    on_close=Callback::new(move |_| set_catalog_open.set(false))
                />
            })}
            <main class="grid gap-[7px] bg-canvas p-[7px] xl:min-h-0 xl:flex-1 xl:overflow-hidden xl:grid-cols-[minmax(0,1fr)_17.5rem]">
                <section class="flex min-h-0 flex-col overflow-hidden border border-border bg-surface" aria-label="AAPL technical chart">
                    <ChartSummary model=model />
                    <AssetChartCanvas model=canvas_model visibility />
                </section>
                <ChartSidebar
                    model=sidebar_model
                    ma20 set_ma20 ma50 set_ma50 ma200 set_ma200 bollinger set_bollinger
                    rsi set_rsi macd set_macd
                />
            </main>
        </div>
    }
}

#[component]
fn ChartHeader(model: AssetChartReadModel) -> impl IntoView {
    let change_class = if model.change_positive {
        "text-finance-positive"
    } else {
        "text-negative-text"
    };
    view! {
        <header class="border-b border-border bg-canvas px-4 pt-3 sm:px-6">
            <div class="flex min-h-[4.75rem] flex-wrap items-start justify-between gap-4 pb-2">
                <div>
                    <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                        <h1 class="numeric text-[1.75rem] font-black leading-none tracking-tight">{model.symbol.clone()}</h1>
                        <span class="text-sm font-medium">{model.name}</span>
                        <span class="text-xs text-text-muted-readable">"•"</span>
                        <span class="text-xs font-medium text-text-secondary">{model.venue}</span>
                    </div>
                    <div class="mt-3 flex items-center gap-3 numeric">
                        <span class="text-[1.625rem] font-semibold leading-none">{model.price}</span>
                        <span class=format!("text-sm font-semibold {change_class}")>{model.absolute_change} " " {model.percentage_change}</span>
                        <span class="h-5 border-l border-border"></span>
                        <span class="text-[0.6875rem] font-semibold text-finance-positive">{model.market_status}</span>
                    </div>
                </div>
                <div class="flex items-center gap-4 pt-1 text-text-secondary">
                    <button type="button" class="grid size-9 place-items-center rounded hover:bg-state-hover" aria-label="Add AAPL to watchlist" title="Add to watchlist">"☆"</button>
                    <button type="button" class="grid size-9 place-items-center rounded hover:bg-state-hover" aria-label="More asset actions" title="More actions">"•••"</button>
                </div>
            </div>
            <AssetTabs ticker=model.symbol capabilities=model.capabilities />
        </header>
    }
}

#[component]
fn ChartToolbar(
    catalog_open: ReadSignal<bool>,
    on_indicators: Callback<()>,
    on_reset: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="dense-scrollbar overflow-x-auto border-b border-border bg-surface">
            <div class="flex h-11 min-w-max items-center gap-1 px-3 text-xs">
                {time_buttons(["1D", "5D", "1M", "3M", "YTD", "1Y", "5Y"], "1D")}
                <span class="mx-2 h-6 border-l border-border"></span>
                {time_buttons(["1m", "5m", "15m", "1h", "1D"], "1D")}
                <span class="mx-2 h-6 border-l border-border"></span>
                <button type="button" class="h-8 rounded px-3 font-medium text-text-primary hover:bg-state-hover">"▥  Candles  ▾"</button>
                <button type="button" class=move || if catalog_open.get() { "h-8 rounded bg-state-selected px-3 font-medium text-interactive-text" } else { "h-8 rounded px-3 font-medium text-text-primary hover:bg-state-hover" } aria-expanded=move || catalog_open.get() on:click=move |_| on_indicators.run(())>"ƒx  Indicators"</button>
                <button type="button" class="h-8 cursor-not-allowed rounded px-3 text-text-secondary opacity-60" disabled>"⊕  Compare"</button>
                <button type="button" class="h-8 cursor-not-allowed rounded px-3 text-text-secondary opacity-60" disabled>"✎  Drawings"</button>
                <button type="button" class="h-8 rounded px-3 text-text-secondary hover:bg-state-hover hover:text-text-primary" on:click=move |_| on_reset.run(())>"↻  Reset"</button>
            </div>
        </div>
    }
}

fn time_buttons<const N: usize>(labels: [&'static str; N], active: &'static str) -> impl IntoView {
    labels
        .into_iter()
        .map(|label| {
            let selected = label == active;
            view! {
                <button type="button" class=if selected {
                    "flex h-11 items-center border-b-2 border-interactive-text px-3 font-semibold text-interactive-text"
                } else {
                    "h-11 border-b-2 border-transparent px-3 font-medium text-text-secondary hover:text-text-primary"
                } aria-pressed=selected>{label}</button>
            }
        })
        .collect_view()
}

#[component]
fn ChartSummary(model: AssetChartReadModel) -> impl IntoView {
    let candle = model.candles.last().cloned();
    view! {
        <div class="flex min-h-10 shrink-0 flex-wrap items-center gap-x-5 gap-y-1 border-b border-border px-3 py-2 text-xs numeric">
            <strong class="text-text-primary">{format!("{} · 1D · {}", model.symbol, model.venue)}</strong>
            {candle.map(|value| view! {
                <span class="text-text-secondary">"O " <span class="text-finance-positive">{format!("{:.2}", value.open)}</span></span>
                <span class="text-text-secondary">"H " <span class="text-finance-positive">{format!("{:.2}", value.high)}</span></span>
                <span class="text-text-secondary">"L " <span class="text-finance-positive">{format!("{:.2}", value.low)}</span></span>
                <span class="text-text-secondary">"C " <span class="text-finance-positive">{format!("{:.2}", value.close)}</span></span>
            })}
            <span class="font-semibold text-finance-positive">{model.absolute_change} " " {model.percentage_change}</span>
        </div>
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn ChartSidebar(
    model: AssetChartReadModel,
    ma20: ReadSignal<bool>,
    set_ma20: WriteSignal<bool>,
    ma50: ReadSignal<bool>,
    set_ma50: WriteSignal<bool>,
    ma200: ReadSignal<bool>,
    set_ma200: WriteSignal<bool>,
    bollinger: ReadSignal<bool>,
    set_bollinger: WriteSignal<bool>,
    rsi: ReadSignal<bool>,
    set_rsi: WriteSignal<bool>,
    macd: ReadSignal<bool>,
    set_macd: WriteSignal<bool>,
) -> impl IntoView {
    let last = model.candles.last().cloned();
    let ma20_value = indicator_value(&model, "ma-20", 0);
    let ma50_value = indicator_value(&model, "ma-50", 0);
    let ma200_value = indicator_value(&model, "ma-200", 0);
    let bollinger_value = indicator_value(&model, "bollinger-bands", 1);
    let rsi_value = indicator_value(&model, "rsi", 0);
    let macd_value = indicator_value(&model, "macd", 0);
    let gex_levels = model.gex_levels.clone();
    view! {
        <aside class="dense-scrollbar min-h-0 overflow-y-auto border border-border bg-surface" aria-label="Indicators and price details">
            <div class="panel-header"><h2 class="text-sm font-semibold">"Indicators"</h2><span class="text-text-secondary">"⚙"</span></div>
            <div class="divide-y divide-border">
                <IndicatorToggle label="MA (20)" value=ma20_value tone="text-interactive-text" enabled=ma20 set_enabled=set_ma20 />
                <IndicatorToggle label="MA (50)" value=ma50_value tone="text-level-special" enabled=ma50 set_enabled=set_ma50 />
                <IndicatorToggle label="MA (200)" value=ma200_value tone="text-interactive-source" enabled=ma200 set_enabled=set_ma200 />
                <IndicatorToggle label="Bollinger (20, 2)" value=bollinger_value tone="text-interactive-text" enabled=bollinger set_enabled=set_bollinger />
                <IndicatorToggle label="RSI (14)" value=rsi_value tone="text-interactive-text" enabled=rsi set_enabled=set_rsi />
                <IndicatorToggle label="MACD (12, 26)" value=macd_value tone="text-finance-positive" enabled=macd set_enabled=set_macd />
            </div>
            <div class="panel-header mt-2"><div><h2 class="text-sm font-semibold">"GEX Levels"</h2><span class="text-[0.625rem] font-semibold uppercase tracking-wider text-level-special">"Mock fixture"</span></div></div>
            <dl>
                {gex_levels.into_iter().map(|level| {
                    let tone = match level.tone {
                        crate::application::asset_chart::ChartTone::Blue => "text-interactive-text",
                        crate::application::asset_chart::ChartTone::Green => "text-finance-positive",
                        crate::application::asset_chart::ChartTone::Red => "text-negative-text",
                        _ => "text-text-primary",
                    };
                    view! { <div class="fact-row min-h-9 py-1.5 text-xs"><dt class="text-text-secondary">{level.label.clone()}</dt><dd class=format!("numeric {tone}")>{format!("{:.2}", level.value)}</dd></div> }
                }).collect_view()}
            </dl>
            <div class="panel-header mt-2"><h2 class="text-sm font-semibold">"Price Details"</h2></div>
            {last.map(|candle| view! {
                <dl>
                    <Metric label="Open" value=format!("{:.2}", candle.open) />
                    <Metric label="High" value=format!("{:.2}", candle.high) />
                    <Metric label="Low" value=format!("{:.2}", candle.low) />
                    <Metric label="Close" value=format!("{:.2}", candle.close) positive=true />
                    <Metric label="Change" value=format!("{} ({})", model.absolute_change, model.percentage_change) positive=true />
                    <Metric label="Volume" value=format_volume(candle.volume) />
                    <Metric label="Avg Volume (20D)" value=format_volume(model.average_volume) />
                </dl>
            })}
        </aside>
    }
}

#[component]
fn IndicatorToggle(
    label: &'static str,
    value: String,
    tone: &'static str,
    enabled: ReadSignal<bool>,
    set_enabled: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <button type="button" class="flex min-h-10 w-full items-center gap-3 px-3 text-left text-xs hover:bg-state-hover" aria-pressed=move || enabled.get() on:click=move |_| set_enabled.update(|value| *value = !*value)>
            <span class=move || if enabled.get() { "text-interactive-text" } else { "text-text-muted-source" } aria-hidden="true">{move || if enabled.get() { "◉" } else { "○" }}</span>
            <span class="flex-1 text-text-secondary">{label}</span>
            <span class=format!("numeric {tone}")>{value}</span>
        </button>
    }
}

fn indicator_value(model: &AssetChartReadModel, id: &str, line_index: usize) -> String {
    model
        .indicators
        .iter()
        .find(|indicator| indicator.id == id)
        .and_then(|indicator| indicator.lines.get(line_index))
        .and_then(|line| line.values.last())
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".into())
}

#[component]
fn Metric(label: &'static str, value: String, #[prop(optional)] positive: bool) -> impl IntoView {
    view! {
        <div class="fact-row min-h-9 py-1.5 text-xs"><dt class="text-text-secondary">{label}</dt><dd class=if positive { "numeric text-finance-positive" } else { "numeric text-text-primary" }>{value}</dd></div>
    }
}

fn format_volume(value: f64) -> String {
    format!("{:.2}M", value / 1_000_000.0)
}

#[component]
fn ChartState(symbol: String, state: FeedbackState, retry: Option<Callback<()>>) -> impl IntoView {
    view! {
        <header class="border-b border-border bg-surface px-4 pt-5 sm:px-6"><h1 class="mb-4 text-2xl font-black numeric">{symbol.clone()}</h1><AssetTabs ticker=symbol /></header>
        <div class="p-4 sm:p-6"><Panel title="Asset Chart"><DataState state />{retry.map(|action| view! { <div class="mt-4 text-center"><button type="button" class="min-h-10 rounded border border-interactive-source bg-state-selected px-4 text-xs font-semibold text-interactive-text" on:click=move |_| action.run(())>"Retry locally"</button></div> })}</Panel></div>
    }
}

#[component]
fn ChartSkeleton() -> impl IntoView {
    view! { <div aria-busy="true" aria-label="Loading asset chart"><div class="h-32 animate-pulse border-b border-border bg-surface"></div><div class="grid gap-2 p-2 xl:grid-cols-[minmax(0,1fr)_17.5rem]"><div class="h-[44rem] animate-pulse border border-border bg-surface"></div><div class="h-[44rem] animate-pulse border border-border bg-surface"></div></div></div> }
}
