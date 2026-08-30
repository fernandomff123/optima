use leptos::prelude::*;

#[component]
#[allow(clippy::too_many_arguments)]
pub fn ChartIndicatorCatalog(
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
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <section class="absolute left-[33rem] top-[8.35rem] z-30 w-80 rounded-panel border border-border bg-surface-elevated shadow-panel" aria-label="Technical indicator catalog">
            <div class="panel-header">
                <div><h2 class="text-sm font-semibold">"Technical Indicators"</h2><p class="text-[0.6875rem] text-text-muted-readable">"Select overlays and lower panels"</p></div>
                <button type="button" class="grid size-8 place-items-center rounded text-text-secondary hover:bg-state-hover hover:text-text-primary" aria-label="Close indicator catalog" on:click=move |_| on_close.run(())>"×"</button>
            </div>
            <div class="p-2">
                <p class="px-2 pb-1 pt-1 text-[0.625rem] font-semibold uppercase tracking-wider text-text-muted-readable">"Overlays"</p>
                <CatalogOption label="Moving Average (20)" enabled=ma20 set_enabled=set_ma20 />
                <CatalogOption label="Moving Average (50)" enabled=ma50 set_enabled=set_ma50 />
                <CatalogOption label="Moving Average (200)" enabled=ma200 set_enabled=set_ma200 />
                <CatalogOption label="Bollinger Bands (20, 2)" enabled=bollinger set_enabled=set_bollinger />
                <p class="px-2 pb-1 pt-3 text-[0.625rem] font-semibold uppercase tracking-wider text-text-muted-readable">"Lower panels"</p>
                <CatalogOption label="Relative Strength Index (14)" enabled=rsi set_enabled=set_rsi />
                <CatalogOption label="MACD (12, 26, 9)" enabled=macd set_enabled=set_macd />
            </div>
        </section>
    }
}

#[component]
fn CatalogOption(
    label: &'static str,
    enabled: ReadSignal<bool>,
    set_enabled: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <button type="button" class=move || if enabled.get() {
            "flex min-h-10 w-full items-center gap-3 rounded bg-state-selected px-3 text-left text-xs text-text-primary"
        } else {
            "flex min-h-10 w-full items-center gap-3 rounded px-3 text-left text-xs text-text-secondary hover:bg-state-hover hover:text-text-primary"
        } aria-pressed=move || enabled.get() on:click=move |_| set_enabled.update(|value| *value = !*value)>
            <span class=move || if enabled.get() { "grid size-4 place-items-center rounded border border-interactive-text bg-interactive-source text-[0.625rem] text-white" } else { "size-4 rounded border border-text-muted-readable" }>{move || enabled.get().then_some("✓").unwrap_or("")}</span>
            <span>{label}</span>
        </button>
    }
}
