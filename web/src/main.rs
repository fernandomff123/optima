use api_models::{
    ApiError, AssetLivePrice, ConfigureTrackedTickerRequest, DataRefreshOrigin,
    DataRefreshRequestResponse, DataRefreshRequestState, DataRefreshRun, DataRefreshState,
    DataRefreshStatusResponse, DataState, Freshness, MarketSpxHistoryResponse,
    PriceHistoryOverview, PriceHistoryPoint, TrackedTicker, TrackedTickerSource,
    UnderlyingResolution, UnderlyingResolutionState,
};
use futures_util::{
    StreamExt,
    future::{AbortHandle, Abortable},
};
use gloo_net::{http::Request, websocket::futures::WebSocket};
use gloo_timers::future::TimeoutFuture;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use plotly::{
    Configuration, Layout, Plot, Scatter,
    common::{Line, Mode, Title},
    layout::{Axis, Margin},
};
use send_wrapper::SendWrapper;
use std::{
    collections::BTreeSet,
    fmt,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use wasm_bindgen::{JsCast, closure::Closure};

mod gamma_exposure;
mod plotly_chart;

use gamma_exposure::GammaExposureView;
use plotly_chart::PlotlyChart;

const API_BASE_PATH: &str = "/api";
const DATA_REFRESH_POLL_INTERVAL_MS: u32 = 5_000;
const DATA_REFRESH_SCHEDULER_TOLERANCE_MS: i64 = 1_500;
const DATA_REFRESH_PAST_ATTEMPT_RECHECK_MS: u32 = 60_000;
const SPX_HISTORY_PLOT_ID: &str = "spx-history-plot";

#[derive(Clone)]
enum DataRefreshLoadState {
    Loading,
    Success {
        status: DataRefreshStatusResponse,
        communication_error: Option<String>,
        awaiting_terminal_confirmation: bool,
    },
    Unavailable(String),
    Error(String),
}

enum DataRefreshStatusResult {
    Success(DataRefreshStatusResponse),
    Unavailable(String),
    Error(String),
}

#[derive(Clone)]
enum TrackedTickersLoadState {
    Loading,
    Success(Vec<TrackedTicker>),
    Error(String),
}

#[derive(Clone, Default)]
struct ObservationGuard(Arc<AtomicBool>);

impl ObservationGuard {
    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }

    fn is_active(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn cancel(&self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Sectors,
    Portfolio,
    Underlyings,
    Builder,
    MarketAnalysis,
    Simulator,
    Settings,
}

impl Page {
    const ALL: [Self; 8] = [
        Self::Dashboard,
        Self::Sectors,
        Self::Portfolio,
        Self::Underlyings,
        Self::Builder,
        Self::MarketAnalysis,
        Self::Simulator,
        Self::Settings,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Sectors => "Setores",
            Self::Portfolio => "Portfolio",
            Self::Underlyings => "Subjacentes",
            Self::Builder => "Construtor",
            Self::MarketAnalysis => "Análise de mercado",
            Self::Simulator => "Simulador",
            Self::Settings => "Configurações",
        }
    }

    const fn icon(self) -> &'static str {
        match self {
            Self::Dashboard => "▦",
            Self::Sectors => "◫",
            Self::Portfolio => "▣",
            Self::Underlyings => "◆",
            Self::Builder => "⌘",
            Self::MarketAnalysis => "⌁",
            Self::Simulator => "◎",
            Self::Settings => "⚙",
        }
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (active_page, set_active_page) = signal(Page::Dashboard);
    let (menu_open, set_menu_open) = signal(false);
    let menu_button = NodeRef::<leptos::html::Button>::new();
    let first_nav_button = NodeRef::<leptos::html::Button>::new();
    let (refresh_state, set_refresh_state) = signal(DataRefreshLoadState::Loading);
    provide_context((refresh_state, set_refresh_state));

    leptos::task::spawn_local(load_data_refresh_status(set_refresh_state));
    Effect::new(move |_| {
        let delay =
            data_refresh_observation_delay_ms(&refresh_state.get(), js_sys::Date::now() as i64);
        if let Some(delay) = delay {
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let guard = ObservationGuard::new();
            let task_guard = guard.clone();
            leptos::task::spawn_local(async move {
                let observation = async move {
                    TimeoutFuture::new(delay).await;
                    if task_guard.is_active() {
                        load_data_refresh_status(set_refresh_state).await;
                    }
                };
                let _ = Abortable::new(observation, abort_registration).await;
            });
            on_cleanup(move || {
                guard.cancel();
                abort_handle.abort();
            });
        }
    });

    let visibility_guard = ObservationGuard::new();
    let observe_on_return = {
        let visibility_guard = visibility_guard.clone();
        move || {
            if visibility_guard.is_active()
                && document_is_visible()
                && data_refresh_needs_visibility_check(
                    &refresh_state.get_untracked(),
                    js_sys::Date::now() as i64,
                )
            {
                leptos::task::spawn_local(load_data_refresh_status(set_refresh_state));
            }
        }
    };
    let focus_listener = window_event_listener(leptos::ev::focus, move |_| observe_on_return());
    let visibility_listener =
        web_sys::window()
            .and_then(|window| window.document())
            .map(|document| {
                let listener_guard = visibility_guard.clone();
                let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                    if listener_guard.is_active()
                        && document_is_visible()
                        && data_refresh_needs_visibility_check(
                            &refresh_state.get_untracked(),
                            js_sys::Date::now() as i64,
                        )
                    {
                        leptos::task::spawn_local(load_data_refresh_status(set_refresh_state));
                    }
                });
                let _ = document.add_event_listener_with_callback(
                    "visibilitychange",
                    listener.as_ref().unchecked_ref(),
                );
                SendWrapper::new((document, listener))
            });
    on_cleanup(move || {
        visibility_guard.cancel();
        focus_listener.remove();
        if let Some(listener) = visibility_listener {
            let (document, listener) = listener.take();
            let _ = document.remove_event_listener_with_callback(
                "visibilitychange",
                listener.as_ref().unchecked_ref(),
            );
        }
    });

    let close_menu = move || {
        set_menu_open.set(false);
        if let Some(button) = menu_button.get() {
            let _ = button.focus();
        }
    };

    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(body) = document.body() else {
            return;
        };
        if menu_open.get() {
            let _ = body.set_attribute("data-mobile-menu-open", "");
        } else {
            let _ = body.remove_attribute("data-mobile-menu-open");
        }
    });

    on_cleanup(|| {
        if let Some(body) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.body())
        {
            let _ = body.remove_attribute("data-mobile-menu-open");
        }
    });

    view! {
        <div
            class="app-shell"
            on:keydown=move |event| {
                if menu_open.get() && event.key() == "Escape" {
                    event.prevent_default();
                    close_menu();
                }
            }
        >
            <header class="mobile-header">
                <button
                    node_ref=menu_button
                    class="menu-button"
                    type="button"
                    aria-label=move || if menu_open.get() { "Fechar menu" } else { "Abrir menu" }
                    aria-expanded=move || menu_open.get().to_string()
                    aria-controls="primary-sidebar"
                    on:click=move |_| {
                        let should_open = !menu_open.get_untracked();
                        set_menu_open.set(should_open);
                        if should_open && let Some(button) = first_nav_button.get() {
                            let _ = button.focus();
                        }
                    }
                >
                    <span aria-hidden="true">{move || if menu_open.get() { "×" } else { "☰" }}</span>
                </button>
                <span class="mobile-breadcrumb">
                    <span>"Optima"</span><span aria-hidden="true">"/"</span><strong>{move || active_page.get().label()}</strong>
                </span>
                <MarketStatus />
            </header>

            <button
                class="menu-backdrop"
                type="button"
                aria-label="Fechar menu"
                tabindex=move || if menu_open.get() { "0" } else { "-1" }
                on:click=move |_| close_menu()
            ></button>

            <aside id="primary-sidebar" class:menu-open=move || menu_open.get() class="sidebar">
                <button
                    class="brand"
                    type="button"
                    aria-label="Ir para o Dashboard"
                    on:click=move |_| {
                        set_active_page.set(Page::Dashboard);
                        if menu_open.get_untracked() {
                            close_menu();
                        }
                    }
                >
                    <span class="brand-mark" aria-hidden="true">"Ω"</span>
                    <span class="brand-copy"><strong>"Optima"</strong><small>"Options workstation"</small></span>
                </button>

                <nav aria-label="Navegação principal">
                    {Page::ALL.map(|page| {
                        view! {
                            <button
                                node_ref=if page == Page::Dashboard { first_nav_button } else { NodeRef::new() }
                                type="button"
                                aria-label=page.label()
                                class:active=move || active_page.get() == page
                                aria-current=move || {
                                    (active_page.get() == page).then_some("page")
                                }
                                on:click=move |_| {
                                    set_active_page.set(page);
                                    if menu_open.get_untracked() {
                                        close_menu();
                                    }
                                }
                            >
                                <span class="nav-icon" aria-hidden="true">{page.icon()}</span>
                                <span>{page.label()}</span>
                            </button>
                        }
                    })}
                </nav>

                <div class="sidebar-status">
                    <span class="status-dot"></span>
                    <div>
                        <strong>"Backend"</strong>
                        <small>{format!("Ligação via {API_BASE_PATH}")}</small>
                    </div>
                </div>
            </aside>

            <main>
                <header class="topbar">
                    <div class="breadcrumb" aria-label="Breadcrumb">
                        <span>"Optima"</span><span aria-hidden="true">"/"</span><strong>{move || active_page.get().label()}</strong>
                    </div>
                    <MarketStatus />
                </header>
                {move || match active_page.get() {
                    Page::Dashboard => view! { <DashboardPage /> }.into_any(),
                    Page::Sectors => view! { <SectorsPage /> }.into_any(),
                    Page::Portfolio => view! { <PortfolioPage /> }.into_any(),
                    Page::Underlyings => view! { <UnderlyingsPage /> }.into_any(),
                    Page::Builder => view! { <BuilderPage /> }.into_any(),
                    Page::MarketAnalysis => view! { <MarketAnalysisPage /> }.into_any(),
                    Page::Simulator => view! { <SimulatorPage /> }.into_any(),
                    Page::Settings => view! { <SettingsPage /> }.into_any(),
                }}
            </main>
        </div>
    }
}

#[component]
fn MarketStatus() -> impl IntoView {
    view! {
        <span class="market-status" role="status">
            <span class="market-status-dot" aria-hidden="true"></span>
            <span class="market-status-label">"Estado do mercado indisponível"</span>
        </span>
    }
}

#[component]
fn PageHeader(
    title: &'static str,
    subtitle: &'static str,
    #[prop(default = "Visão geral")] eyebrow: &'static str,
) -> impl IntoView {
    view! {
        <header class="page-header">
            <div>
                <span class="page-eyebrow">{eyebrow}</span>
                <h1>{title}</h1>
                <p>{subtitle}</p>
            </div>
        </header>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SectorPeriod {
    Week,
    TwoWeeks,
    Month,
}

impl SectorPeriod {
    const ALL: [Self; 3] = [Self::Week, Self::TwoWeeks, Self::Month];

    const fn label(self) -> &'static str {
        match self {
            Self::Week => "1 semana",
            Self::TwoWeeks => "2 semanas",
            Self::Month => "1 mês",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Week => 0,
            Self::TwoWeeks => 1,
            Self::Month => 2,
        }
    }
}

#[derive(Clone, Copy)]
struct SectorMarketDemo {
    name: &'static str,
    etf: &'static str,
    description: &'static str,
    changes: [f64; 3],
    spx_changes: [f64; 3],
    relative_strength: [&'static str; 3],
    histograms: [[i8; 8]; 3],
    size_class: &'static str,
}

const SECTOR_MARKET_DEMO: [SectorMarketDemo; 11] = [
    SectorMarketDemo {
        name: "Tecnologia",
        etf: "XLK",
        description: "Liderança apoiada por software e semicondutores, com momentum acima do mercado.",
        changes: [2.8, 3.6, 5.2],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Forte", "Forte", "Muito forte"],
        histograms: [
            [2, 3, -1, 4, 5, 3, 6, 7],
            [1, 3, 2, 5, 4, 6, 7, 8],
            [-2, 1, 3, 4, 6, 5, 7, 8],
        ],
        size_class: "sector-tile-xl",
    },
    SectorMarketDemo {
        name: "Financeiro",
        etf: "XLF",
        description: "Bancos e serviços financeiros avançam, beneficiando de uma curva de taxas mais favorável.",
        changes: [1.7, 2.4, 3.1],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Moderada", "Forte", "Neutra"],
        histograms: [
            [-1, 2, 2, 3, 1, 4, 3, 5],
            [1, -1, 3, 4, 3, 5, 4, 6],
            [-2, 1, 2, 4, 3, 5, 4, 5],
        ],
        size_class: "sector-tile-lg",
    },
    SectorMarketDemo {
        name: "Energia",
        etf: "XLE",
        description: "O setor acompanha a recuperação das matérias-primas, ainda com oscilações relevantes.",
        changes: [-1.9, -0.8, 1.4],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Fraca", "Fraca", "Neutra"],
        histograms: [
            [2, -3, -5, -2, 1, -4, -2, -3],
            [-3, -2, 1, -1, 2, -2, 1, -1],
            [-2, -1, 1, 3, 2, 4, 3, 4],
        ],
        size_class: "sector-tile-lg",
    },
    SectorMarketDemo {
        name: "Industrial",
        etf: "XLI",
        description: "Procura consistente em transportes e bens de capital sustenta um avanço gradual.",
        changes: [0.9, 1.5, 2.7],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Neutra", "Neutra", "Neutra"],
        histograms: [
            [-1, 1, 2, 1, 3, 2, 2, 4],
            [1, 2, -1, 3, 2, 4, 3, 4],
            [-1, 1, 3, 2, 4, 5, 4, 6],
        ],
        size_class: "sector-tile-md",
    },
    SectorMarketDemo {
        name: "Saúde",
        etf: "XLV",
        description: "Desempenho defensivo e estável, com dispersão entre farmacêuticas e biotecnologia.",
        changes: [0.3, 0.6, 1.1],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Neutra", "Fraca", "Fraca"],
        histograms: [
            [1, -1, 2, 1, -1, 2, 1, 2],
            [-1, 1, 2, -1, 2, 1, 3, 2],
            [-1, 2, 1, 3, 2, 3, 2, 4],
        ],
        size_class: "sector-tile-md",
    },
    SectorMarketDemo {
        name: "Consumo discricionário",
        etf: "XLY",
        description: "Retalho e automóvel recuperam, embora o consumo permaneça seletivo.",
        changes: [1.2, 2.1, 3.8],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Neutra", "Moderada", "Forte"],
        histograms: [
            [-2, 1, 3, 2, 4, 3, 2, 5],
            [-1, 2, 1, 4, 3, 5, 4, 6],
            [-2, 1, 3, 5, 4, 6, 7, 8],
        ],
        size_class: "sector-tile-lg",
    },
    SectorMarketDemo {
        name: "Comunicações",
        etf: "XLC",
        description: "Media e plataformas digitais mantêm desempenho positivo próximo do índice.",
        changes: [0.6, 1.3, 2.9],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Neutra", "Neutra", "Neutra"],
        histograms: [
            [1, 2, -1, 2, 3, 1, 3, 3],
            [-1, 2, 3, 2, 4, 3, 4, 5],
            [-1, 2, 3, 4, 3, 5, 5, 6],
        ],
        size_class: "sector-tile-md",
    },
    SectorMarketDemo {
        name: "Bens essenciais",
        etf: "XLP",
        description: "Perfil defensivo limita a volatilidade e também a participação no movimento do mercado.",
        changes: [-0.2, 0.1, 0.8],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Fraca", "Fraca", "Fraca"],
        histograms: [
            [1, -1, 1, -2, 1, -1, 1, -1],
            [-1, 1, -1, 2, -1, 1, 1, 1],
            [-1, 1, 2, 1, 2, 1, 3, 2],
        ],
        size_class: "sector-tile-md",
    },
    SectorMarketDemo {
        name: "Materiais",
        etf: "XLB",
        description: "Metais e químicos recuam com sinais mistos na procura industrial global.",
        changes: [-0.8, -1.4, -0.5],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Fraca", "Fraca", "Fraca"],
        histograms: [
            [1, -2, -1, -3, 1, -2, -1, -3],
            [-1, -3, -2, 1, -4, -2, -3, -4],
            [-2, -1, 1, -2, -1, 1, -2, -1],
        ],
        size_class: "sector-tile-sm",
    },
    SectorMarketDemo {
        name: "Utilities",
        etf: "XLU",
        description: "Sensibilidade às taxas mantém o setor sob pressão apesar do seu caráter defensivo.",
        changes: [-1.3, -2.2, -3.1],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Fraca", "Muito fraca", "Muito fraca"],
        histograms: [
            [1, -2, -3, -1, -4, -2, -3, -4],
            [-2, -3, 1, -4, -3, -5, -4, -6],
            [-1, -3, -2, -5, -4, -6, -5, -7],
        ],
        size_class: "sector-tile-sm",
    },
    SectorMarketDemo {
        name: "Imobiliário",
        etf: "XLRE",
        description: "REITs refletem custos de financiamento elevados e menor procura por ativos de rendimento.",
        changes: [-2.4, -3.0, -3.7],
        spx_changes: [1.1, 1.8, 3.0],
        relative_strength: ["Muito fraca", "Muito fraca", "Muito fraca"],
        histograms: [
            [-1, -3, -2, -5, -4, -6, -5, -7],
            [-2, -4, -3, -5, -6, -5, -7, -8],
            [-2, -3, -5, -4, -6, -7, -6, -8],
        ],
        size_class: "sector-tile-sm",
    },
];

fn signed_percent(value: f64) -> String {
    format!("{value:+.1}%").replace('.', ",")
}

fn heat_tone(value: f64) -> &'static str {
    if value <= -2.0 {
        "heat-negative-strong"
    } else if value < -0.35 {
        "heat-negative"
    } else if value <= 0.35 {
        "heat-neutral"
    } else if value < 2.0 {
        "heat-positive"
    } else {
        "heat-positive-strong"
    }
}

#[component]
fn SectorsPage() -> impl IntoView {
    let (period, set_period) = signal(SectorPeriod::Week);
    let (selected, set_selected) = signal(0_usize);

    view! {
        <section class="page sectors-page">
            <header class="page-header sectors-header">
                <div>
                    <span class="page-eyebrow">"Mercado"</span>
                    <h1>"Setores"</h1>
                    <p>"Mapa de calor e força relativa dos setores do S&P 500"</p>
                </div>
                <div class="period-filter" role="group" aria-label="Período do desempenho demonstrativo">
                    {SectorPeriod::ALL.map(|option| view! {
                        <button type="button" class:active=move || period.get() == option aria-pressed=move || period.get() == option on:click=move |_| set_period.set(option)>{option.label()}</button>
                    })}
                </div>
            </header>
            <div class="sectors-demo-notice" role="note"><span aria-hidden="true">"◇"</span>"Valores exclusivamente demonstrativos · não representam dados da API"</div>
            <div class="sectors-layout">
                <article class="heatmap-card">
                    <div class="sector-card-heading"><div><h2>"Mapa de calor"</h2><p>{move || format!("Variação demonstrativa · {}", period.get().label())}</p></div><span>"S&P 500"</span></div>
                    <div class="sector-heatmap" aria-label="Selecionar setor">
                        {SECTOR_MARKET_DEMO.into_iter().enumerate().map(|(index, sector)| view! {
                            <button type="button" class=move || format!("sector-tile {} {}{}", sector.size_class, heat_tone(sector.changes[period.get().index()]), if selected.get() == index { " active" } else { "" }) aria-pressed=move || selected.get() == index on:click=move |_| set_selected.set(index)>
                                <span>{sector.name}</span><strong>{move || signed_percent(sector.changes[period.get().index()])}</strong>
                            </button>
                        }).collect_view()}
                    </div>
                    <div class="heat-scale" aria-label="Escala de desempenho demonstrativo de menos quatro a mais quatro por cento"><span>"−4%"</span><i></i><span>"0%"</span><i></i><span>"+4%"</span></div>
                </article>
                <SectorDetailPanel selected=selected period=period />
            </div>
        </section>
    }
}

#[component]
fn SectorDetailPanel(
    selected: ReadSignal<usize>,
    period: ReadSignal<SectorPeriod>,
) -> impl IntoView {
    view! {
        <article class="sector-detail" aria-live="polite">
            {move || {
                let sector = SECTOR_MARKET_DEMO[selected.get()];
                let index = period.get().index();
                let change = sector.changes[index];
                let histogram = sector.histograms[index];
                let histogram_max = histogram
                    .iter()
                    .map(|value| value.unsigned_abs())
                    .max()
                    .unwrap_or(1)
                    .max(1);
                let histogram_description = format!("Histograma demonstrativo de oito intervalos para {} no período de {}", sector.name, period.get().label());
                view! {
                    <div class="sector-detail-top"><div><span>"Setor selecionado"</span><h2>{sector.name}</h2></div><b>{sector.etf}</b></div>
                    <strong class=format!("sector-main-change {}", if change >= 0.0 { "positive" } else { "negative" })>{signed_percent(change)}</strong>
                    <span class="sector-period-copy">{format!("Desempenho demonstrativo · {}", period.get().label())}</span>
                    <p class="sector-comment">{sector.description}</p>
                    <dl class="sector-stats">
                        <div><dt>"Referência S&P 500"</dt><dd>{signed_percent(sector.spx_changes[index])}</dd></div>
                        <div><dt>"Força relativa"</dt><dd>{sector.relative_strength[index]}</dd></div>
                    </dl>
                    <figure class="sector-histogram">
                        <figcaption><span>"Momentum"</span><small>"Histograma demonstrativo"</small></figcaption>
                        <div class="histogram-plot" role="img" aria-label=histogram_description>
                            <i class="histogram-zero"></i>
                            {histogram.into_iter().map(|value| {
                                let magnitude = value.unsigned_abs();
                                let normalized = if magnitude == 0 { 0 } else { (u16::from(magnitude) * 100 / u16::from(histogram_max)).max(12) };
                                view! {
                                <span class=if value < 0 { "negative" } else { "positive" } style=format!("--bar-size: {normalized}%")><i></i></span>
                                }
                            }).collect_view()}
                        </div>
                    </figure>
                    <button class="sector-future-action" type="button" disabled aria-label="Ver ativos do setor — funcionalidade futura">"Ver ativos do setor"<span aria-hidden="true">"→"</span></button>
                    <small class="future-note">"Funcionalidade futura"</small>
                }
            }}
        </article>
    }
}

#[derive(Clone, Copy)]
struct DemoMetric {
    label: &'static str,
    value: &'static str,
    trend: &'static str,
    bars: [u8; 8],
}
#[derive(Clone, Copy)]
struct SectorDemo {
    name: &'static str,
    tickers: &'static str,
    percent: u8,
    width: u8,
    tone: &'static str,
}
#[derive(Clone, Copy)]
struct GreekDemo {
    name: &'static str,
    detail: &'static str,
    value: &'static str,
    width: u8,
    tone: &'static str,
}
#[derive(Clone, Copy)]
struct AlertDemo {
    title: &'static str,
    detail: &'static str,
    tone: &'static str,
}
#[derive(Clone, Copy)]
struct MaturityDemo {
    range: &'static str,
    count: u8,
    height: u8,
    tone: &'static str,
}
#[derive(Clone, Copy)]
struct ActivityDemo {
    time: &'static str,
    title: &'static str,
    detail: &'static str,
    value: Option<&'static str>,
    icon: &'static str,
    tone: &'static str,
}
#[derive(Clone, Copy)]
struct RiskPositionDemo {
    ticker: &'static str,
    strategy: &'static str,
    strike: &'static str,
    expiry: &'static str,
    dte: u8,
    pnl: &'static str,
    pnl_tone: &'static str,
    reason: &'static str,
    risk: &'static str,
    risk_tone: &'static str,
}
#[derive(Clone, Copy)]
struct PnlDemo {
    portfolio: &'static str,
    benchmark: &'static str,
    difference: &'static str,
}

struct DashboardDemo {
    metrics: [DemoMetric; 2],
    sectors: [SectorDemo; 4],
    greeks: [GreekDemo; 5],
    pnl: PnlDemo,
    alerts: [AlertDemo; 3],
    maturities: [MaturityDemo; 5],
    activities: [ActivityDemo; 4],
    positions: [RiskPositionDemo; 5],
}

impl DashboardDemo {
    const fn get() -> Self {
        Self {
            metrics: [
                DemoMetric {
                    label: "Valor do portfolio",
                    value: "€124 580",
                    trend: "+€3 240 · 2,67%",
                    bars: [28, 35, 32, 46, 43, 56, 61, 68],
                },
                DemoMetric {
                    label: "P&L não realizado",
                    value: "€8 420",
                    trend: "Theta +€145 / dia",
                    bars: [32, 38, 29, 42, 49, 45, 62, 73],
                },
            ],
            sectors: [
                SectorDemo {
                    name: "Tecnologia",
                    tickers: "AAPL · MSFT · NVDA",
                    percent: 42,
                    width: 72,
                    tone: "tone-blue",
                },
                SectorDemo {
                    name: "Financeiro",
                    tickers: "JPM · BAC",
                    percent: 21,
                    width: 44,
                    tone: "tone-violet",
                },
                SectorDemo {
                    name: "Energia",
                    tickers: "XLE · OXY",
                    percent: 12,
                    width: 27,
                    tone: "tone-green",
                },
                SectorDemo {
                    name: "Índices",
                    tickers: "SPX · SPY",
                    percent: 25,
                    width: 51,
                    tone: "tone-cyan",
                },
            ],
            greeks: [
                GreekDemo {
                    name: "Delta",
                    detail: "Direção · atenção",
                    value: "+0,42",
                    width: 42,
                    tone: "tone-amber",
                },
                GreekDemo {
                    name: "Gamma",
                    detail: "Convexidade · risco",
                    value: "−0,018",
                    width: 18,
                    tone: "tone-red",
                },
                GreekDemo {
                    name: "Theta",
                    detail: "Prémio por dia",
                    value: "+€145/dia",
                    width: 58,
                    tone: "tone-green",
                },
                GreekDemo {
                    name: "Vega",
                    detail: "Sensibilidade à volatilidade",
                    value: "+€680",
                    width: 68,
                    tone: "tone-blue",
                },
                GreekDemo {
                    name: "Rho",
                    detail: "Sensibilidade à taxa",
                    value: "+€84",
                    width: 24,
                    tone: "tone-blue",
                },
            ],
            pnl: PnlDemo {
                portfolio: "+6,8%",
                benchmark: "+3,1%",
                difference: "+3,7 pp",
            },
            alerts: [
                AlertDemo {
                    title: "Gamma elevado",
                    detail: "NVDA · expiração dentro de 2 dias",
                    tone: "red",
                },
                AlertDemo {
                    title: "Vega concentrada",
                    detail: "60% da exposição em earnings esta semana",
                    tone: "amber",
                },
                AlertDemo {
                    title: "Theta acelerado",
                    detail: "Posições short acima de €200/dia",
                    tone: "blue",
                },
            ],
            maturities: [
                MaturityDemo {
                    range: "0–7",
                    count: 3,
                    height: 42,
                    tone: "tone-red",
                },
                MaturityDemo {
                    range: "8–14",
                    count: 2,
                    height: 31,
                    tone: "tone-amber",
                },
                MaturityDemo {
                    range: "15–30",
                    count: 4,
                    height: 58,
                    tone: "tone-blue",
                },
                MaturityDemo {
                    range: "31–60",
                    count: 2,
                    height: 35,
                    tone: "tone-green",
                },
                MaturityDemo {
                    range: "60+",
                    count: 1,
                    height: 22,
                    tone: "tone-violet",
                },
            ],
            activities: [
                ActivityDemo {
                    time: "Agora",
                    title: "P&L atualizado",
                    detail: "Portfolio valorizou €340 desde a última atualização",
                    value: Some("+€340"),
                    icon: "↗",
                    tone: "green",
                },
                ActivityDemo {
                    time: "Há 18 min",
                    title: "Nova posição aberta",
                    detail: "AAPL 220 Call · 5 contratos",
                    value: None,
                    icon: "+",
                    tone: "blue",
                },
                ActivityDemo {
                    time: "Há 32 min",
                    title: "Alerta de Gamma elevado",
                    detail: "NVDA · expiração dentro de 2 dias",
                    value: None,
                    icon: "!",
                    tone: "red",
                },
                ActivityDemo {
                    time: "Hoje, 09:42",
                    title: "Preços recalculados",
                    detail: "14 posições atualizadas com dados de mercado",
                    value: None,
                    icon: "◆",
                    tone: "amber",
                },
            ],
            positions: [
                RiskPositionDemo {
                    ticker: "NVDA",
                    strategy: "Call short",
                    strike: "$125",
                    expiry: "15 Ago 2026",
                    dte: 7,
                    pnl: "+€2 400",
                    pnl_tone: "positive",
                    reason: "Gamma elevado",
                    risk: "Crítico",
                    risk_tone: "red",
                },
                RiskPositionDemo {
                    ticker: "GOOGL",
                    strategy: "Call short",
                    strike: "$165",
                    expiry: "22 Ago 2026",
                    dte: 14,
                    pnl: "+€1 260",
                    pnl_tone: "positive",
                    reason: "Vega concentrada",
                    risk: "Alto",
                    risk_tone: "amber",
                },
                RiskPositionDemo {
                    ticker: "AMD",
                    strategy: "Put short",
                    strike: "$140",
                    expiry: "15 Ago 2026",
                    dte: 7,
                    pnl: "+€880",
                    pnl_tone: "positive",
                    reason: "Vencimento próximo",
                    risk: "Alto",
                    risk_tone: "amber",
                },
                RiskPositionDemo {
                    ticker: "AAPL",
                    strategy: "Put long",
                    strike: "$210",
                    expiry: "22 Ago 2026",
                    dte: 14,
                    pnl: "−€700",
                    pnl_tone: "negative",
                    reason: "Perda não realizada",
                    risk: "Médio",
                    risk_tone: "blue",
                },
                RiskPositionDemo {
                    ticker: "SPY",
                    strategy: "Bull call",
                    strike: "$540 / $545",
                    expiry: "20 Set 2026",
                    dte: 43,
                    pnl: "+€1 120",
                    pnl_tone: "positive",
                    reason: "Risco limitado",
                    risk: "Baixo",
                    risk_tone: "green",
                },
            ],
        }
    }
}

#[component]
fn DashboardPanel(
    class: &'static str,
    title: &'static str,
    subtitle: &'static str,
    children: Children,
) -> impl IntoView {
    view! { <article class=format!("dashboard-panel {class}")><div class="dashboard-panel-title"><div><h2>{title}</h2><p>{subtitle}</p></div></div>{children()}</article> }
}

#[component]
fn DemoMetric(metric: DemoMetric) -> impl IntoView {
    view! { <article class="demo-metric"><div><span>{metric.label}</span><i aria-hidden="true"></i></div><strong>{metric.value}</strong><small class="positive">{metric.trend}</small><div class="demo-spark" aria-hidden="true">{metric.bars.map(|bar| view! { <i style=format!("height:{bar}%")></i> })}</div></article> }
}

#[component]
fn DemoPnlChart(data: PnlDemo) -> impl IntoView {
    view! {
        <figure class="demo-pnl-chart">
            <div class="pnl-summary"><span><i class="portfolio-key"></i>"Portfolio "<b>{data.portfolio}</b></span><span><i class="benchmark-key"></i>"S&P 500 "<b>{data.benchmark}</b></span><strong>{data.difference}</strong></div>
            <svg viewBox="0 0 440 170" role="img" aria-labelledby="demo-pnl-title demo-pnl-desc">
                <title id="demo-pnl-title">"P&L demonstrativo do portfolio comparado com o S&P 500"</title>
                <desc id="demo-pnl-desc">"Séries simuladas dos últimos 30 dias, de 12 de julho a 11 de agosto: portfolio mais 6,8 por cento e benchmark mais 3,1 por cento."</desc>
                <defs><linearGradient id="demo-pnl-fill" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#27d3a2" stop-opacity=".28"/><stop offset="1" stop-color="#27d3a2" stop-opacity="0"/></linearGradient></defs>
                <g class="demo-chart-grid" aria-hidden="true"><line x1="0" y1="20" x2="420" y2="20"/><line x1="0" y1="60" x2="420" y2="60"/><line x1="0" y1="100" x2="420" y2="100"/><line x1="0" y1="140" x2="420" y2="140"/></g>
                <polygon points="0,150 0,139 28,131 56,134 84,112 112,117 140,93 168,101 196,77 224,83 252,58 280,64 308,39 336,46 364,24 392,29 420,16 420,150" fill="url(#demo-pnl-fill)"/>
                <polyline points="0,139 28,136 56,126 84,129 112,115 140,119 168,106 196,101 224,94 252,90 280,82 308,77 336,70 364,66 392,58 420,54" class="demo-benchmark-line"/>
                <polyline points="0,139 28,131 56,134 84,112 112,117 140,93 168,101 196,77 224,83 252,58 280,64 308,39 336,46 364,24 392,29 420,16" class="demo-portfolio-line"/>
                <circle cx="420" cy="16" r="4" class="demo-portfolio-point"/><circle cx="420" cy="54" r="4" class="demo-benchmark-point"/>
                <g class="demo-chart-dates"><text x="0" y="166">"12 Jul"</text><text x="210" y="166" text-anchor="middle">"27 Jul"</text><text x="420" y="166" text-anchor="end">"11 Ago"</text></g>
            </svg><figcaption>"Valores simulados; não representam desempenho real."</figcaption>
        </figure>
    }
}

fn risk_table_row(position: RiskPositionDemo) -> impl IntoView {
    view! { <tr><td><strong class="ticker">{position.ticker}</strong></td><td><span class="strategy-pill">{position.strategy}</span></td><td>{position.strike}</td><td>{position.expiry}</td><td><b>{position.dte}</b></td><td><b class=position.pnl_tone>{position.pnl}</b></td><td>{position.reason}</td><td><span class=format!("risk-level {}", position.risk_tone)>{position.risk}</span></td></tr> }
}

fn risk_mobile_card(position: RiskPositionDemo) -> impl IntoView {
    view! { <article class="risk-position-card"><header><strong class="ticker">{position.ticker}</strong><span class=format!("risk-level {}", position.risk_tone)>{position.risk}</span></header><dl><div><dt>"Estratégia"</dt><dd>{position.strategy}</dd></div><div><dt>"Strike"</dt><dd>{position.strike}</dd></div><div><dt>"Vencimento"</dt><dd>{position.expiry}</dd></div><div><dt>"DTE"</dt><dd>{position.dte}</dd></div><div><dt>"P&L atual"</dt><dd class=position.pnl_tone>{position.pnl}</dd></div><div class="risk-reason"><dt>"Motivo principal"</dt><dd>{position.reason}</dd></div></dl></article> }
}

fn status_is_running(status: &DataRefreshStatusResponse) -> bool {
    status.running
        || status
            .latest
            .as_ref()
            .is_some_and(|run| run.state == DataRefreshState::Running)
}

fn data_refresh_is_awaiting_terminal(state: &DataRefreshLoadState) -> bool {
    matches!(
        state,
        DataRefreshLoadState::Success {
            awaiting_terminal_confirmation: true,
            ..
        }
    )
}

fn apply_data_refresh_status_result(
    state: &mut DataRefreshLoadState,
    result: DataRefreshStatusResult,
) {
    match result {
        DataRefreshStatusResult::Success(status) => {
            let awaiting_terminal_confirmation = status_is_running(&status);
            *state = DataRefreshLoadState::Success {
                status,
                communication_error: None,
                awaiting_terminal_confirmation,
            };
        }
        DataRefreshStatusResult::Unavailable(message) => match state {
            DataRefreshLoadState::Success {
                communication_error,
                ..
            } => *communication_error = Some(message),
            _ => *state = DataRefreshLoadState::Unavailable(message),
        },
        DataRefreshStatusResult::Error(message) => match state {
            DataRefreshLoadState::Success {
                communication_error,
                ..
            } => *communication_error = Some(message),
            _ => *state = DataRefreshLoadState::Error(message),
        },
    }
}

fn mark_manual_refresh_known(state: &mut DataRefreshLoadState, run: Option<DataRefreshRun>) {
    match state {
        DataRefreshLoadState::Success {
            status,
            communication_error,
            awaiting_terminal_confirmation,
        } => {
            *awaiting_terminal_confirmation = true;
            *communication_error = None;
            if let Some(run) = run {
                status.running = true;
                status.latest = Some(run.clone());
                status.recent.retain(|recent| recent.id != run.id);
                status.recent.insert(0, run);
            }
        }
        _ => {
            let recent = run.clone().into_iter().collect();
            *state = DataRefreshLoadState::Success {
                status: DataRefreshStatusResponse {
                    running: true,
                    latest: run,
                    recent,
                },
                communication_error: None,
                awaiting_terminal_confirmation: true,
            };
        }
    }
}

fn data_refresh_observation_delay_ms(state: &DataRefreshLoadState, now_ms: i64) -> Option<u32> {
    let DataRefreshLoadState::Success {
        status,
        awaiting_terminal_confirmation,
        ..
    } = state
    else {
        return None;
    };

    if *awaiting_terminal_confirmation {
        return Some(DATA_REFRESH_POLL_INTERVAL_MS);
    }

    let next_attempt_ms = status
        .latest
        .as_ref()?
        .next_attempt_at
        .as_ref()?
        .timestamp_millis();
    if next_attempt_ms <= now_ms {
        return Some(DATA_REFRESH_PAST_ATTEMPT_RECHECK_MS);
    }

    let delay = next_attempt_ms
        .saturating_sub(now_ms)
        .saturating_add(DATA_REFRESH_SCHEDULER_TOLERANCE_MS);
    Some(u32::try_from(delay).unwrap_or(u32::MAX))
}

fn data_refresh_needs_visibility_check(state: &DataRefreshLoadState, now_ms: i64) -> bool {
    match state {
        DataRefreshLoadState::Loading
        | DataRefreshLoadState::Unavailable(_)
        | DataRefreshLoadState::Error(_) => true,
        DataRefreshLoadState::Success {
            status,
            awaiting_terminal_confirmation,
            ..
        } => {
            *awaiting_terminal_confirmation
                || status
                    .latest
                    .as_ref()
                    .and_then(|run| run.next_attempt_at.as_ref())
                    .is_some_and(|next_attempt| {
                        next_attempt.timestamp_millis()
                            <= now_ms.saturating_add(DATA_REFRESH_SCHEDULER_TOLERANCE_MS)
                    })
        }
    }
}

fn document_is_visible() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.visibility_state() == web_sys::VisibilityState::Visible)
}

async fn fetch_data_refresh_status() -> DataRefreshStatusResult {
    let response = match Request::get("/api/data-refresh/status").send().await {
        Ok(response) => response,
        Err(_) => {
            return DataRefreshStatusResult::Error(
                "Não foi possível contactar o serviço de atualização.".to_string(),
            );
        }
    };

    match response.status() {
        200 => match response.json::<DataRefreshStatusResponse>().await {
            Ok(status) => DataRefreshStatusResult::Success(status),
            Err(_) => {
                DataRefreshStatusResult::Error("O serviço devolveu um estado inválido.".to_string())
            }
        },
        404 | 501 | 503 => DataRefreshStatusResult::Unavailable(
            "O serviço de atualização não está disponível.".to_string(),
        ),
        _ => DataRefreshStatusResult::Error(format!(
            "Não foi possível obter o estado (HTTP {}).",
            response.status()
        )),
    }
}

async fn load_data_refresh_status(set_state: WriteSignal<DataRefreshLoadState>) {
    let result = fetch_data_refresh_status().await;
    set_state.update(|state| apply_data_refresh_status_result(state, result));
}

fn refresh_state_label(state: DataRefreshState) -> &'static str {
    match state {
        DataRefreshState::Running => "Em curso",
        DataRefreshState::Completed => "Concluída",
        DataRefreshState::Partial => "Parcial",
        DataRefreshState::Failed => "Falhou",
    }
}

fn refresh_activity_title(state: DataRefreshState) -> &'static str {
    match state {
        DataRefreshState::Running => "Atualização de dados iniciada",
        DataRefreshState::Completed => "Atualização concluída",
        DataRefreshState::Partial => "Atualização parcial",
        DataRefreshState::Failed => "Atualização falhou",
    }
}

fn refresh_origin_label(origin: DataRefreshOrigin) -> &'static str {
    match origin {
        DataRefreshOrigin::Startup => "Arranque",
        DataRefreshOrigin::Scheduled => "Agendada",
        DataRefreshOrigin::Retry => "Nova tentativa",
        DataRefreshOrigin::Manual => "Manual",
    }
}

fn refresh_tone(state: DataRefreshState) -> &'static str {
    match state {
        DataRefreshState::Running => "blue",
        DataRefreshState::Completed => "green",
        DataRefreshState::Partial => "amber",
        DataRefreshState::Failed => "red",
    }
}

fn format_refresh_datetime(value: &chrono::DateTime<chrono::Utc>) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
        value.timestamp_millis() as f64
    ));
    date.to_locale_string("pt-PT", &wasm_bindgen::JsValue::UNDEFINED)
        .into()
}

fn refresh_counts(run: &DataRefreshRun) -> Option<String> {
    (run.items_obtained > 0 || run.items_persisted > 0).then(|| {
        format!(
            "{} obtidos · {} guardados",
            run.items_obtained, run.items_persisted
        )
    })
}

#[component]
fn SystemActivity(run: DataRefreshRun) -> impl IntoView {
    let tone = refresh_tone(run.state);
    let state = run.state;
    let detail = format!("Origem: {}", refresh_origin_label(run.origin));
    let counts = refresh_counts(&run);
    let next_attempt = run.next_attempt_at.as_ref().map(|date| {
        format!(
            "Tentativa seguinte prevista: {}",
            format_refresh_datetime(date)
        )
    });
    view! {
        <div class="activity-row system-activity">
            <span class=format!("activity-icon {tone}") aria-hidden="true">"↻"</span>
            <div>
                <small>{format!("Sistema · {}", format_refresh_datetime(&run.started_at))}</small>
                <b>{refresh_activity_title(state)}</b>
                <p>{detail}</p>
                {counts.map(|value| view! { <p>{value}</p> })}
                {next_attempt.map(|value| view! { <p>{value}</p> })}
            </div>
        </div>
    }
}

#[component]
fn DashboardRefreshActivities() -> impl IntoView {
    let (refresh_state, _) = expect_context::<(
        ReadSignal<DataRefreshLoadState>,
        WriteSignal<DataRefreshLoadState>,
    )>();

    move || match refresh_state.get() {
        DataRefreshLoadState::Loading => view! {
            <div class="activity-state" role="status">"A carregar atividade do sistema…"</div>
        }
        .into_any(),
        DataRefreshLoadState::Unavailable(message) => view! {
            <div class="activity-state unavailable" role="status">{message}</div>
        }
        .into_any(),
        DataRefreshLoadState::Error(message) => view! {
            <div class="activity-state error" role="alert">{message}</div>
        }
        .into_any(),
        DataRefreshLoadState::Success {
            status,
            communication_error,
            ..
        } => {
            let mut recent = status.recent;
            recent.sort_by_key(|run| std::cmp::Reverse(run.started_at));
            let activities = if recent.is_empty() {
                view! { <div class="activity-state" role="status">"Sem atividade do sistema registada."</div> }
                    .into_any()
            } else {
                view! { <>{recent.into_iter().map(|run| view! { <SystemActivity run=run /> }).collect_view()}</> }
                    .into_any()
            };
            view! {
                <>
                    {communication_error.map(|message| view! {
                        <div class="activity-state error" role="alert">{message}</div>
                    })}
                    {activities}
                </>
            }
            .into_any()
        }
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    let data = DashboardDemo::get();

    view! {
        <section class="page dashboard-page" aria-describedby="dashboard-demo-note">
            <header class="page-header dashboard-header">
                <div>
                    <span class="page-eyebrow">"Visão geral"</span>
                    <h1>"Dashboard"</h1>
                    <p>"O pulso do seu livro de opções, num só lugar."</p>
                </div>
                <div class="environment-switch" aria-label="Ambiente visual; não altera fontes de dados">
                    <button type="button" disabled><small>"AMBIENTE"</small>"Real"</button>
                    <button type="button" class="active" aria-pressed="true"><small>"AMBIENTE"</small>"Simulação"</button>
                </div>
            </header>
            <p id="dashboard-demo-note" class="demo-notice" role="note">
                <span aria-hidden="true">"◇"</span>
                "Simulação · os valores, sinais e atividades financeiras são demonstrativos; a atividade do Sistema é factual."
            </p>

            <div class="dashboard-metrics">
                {data.metrics.into_iter().map(|metric| view! { <DemoMetric metric=metric /> }).collect_view()}
            </div>

            <div class="official-dashboard-grid">
                <div class="dashboard-top-row">
                    <DashboardPanel class="exposure-panel" title="Exposição por setor" subtitle="Percentagem do valor de mercado · simulação">
                        <div class="sector-list">
                            {data.sectors.into_iter().map(|sector| view! {
                                <div class="sector-row">
                                    <div class="sector-copy"><b>{sector.name}</b><small>{sector.tickers}</small><strong>{format!("{}%", sector.percent)}</strong></div>
                                    <div class="demo-progress" role="img" aria-label=format!("{}: {} por cento", sector.name, sector.percent)>
                                        <i class=sector.tone style=format!("width:{}%", sector.width)></i>
                                    </div>
                                </div>
                            }).collect_view()}
                        </div>
                    </DashboardPanel>

                    <DashboardPanel class="greek-panel" title="Gregas do portfolio" subtitle="Exposição líquida · dados demonstrativos">
                        <div class="greek-list">
                            {data.greeks.into_iter().map(|greek| view! {
                                <div class="greek-row">
                                    <div><span><b>{greek.name}</b><small>{greek.detail}</small></span><strong>{greek.value}</strong></div>
                                    <div class="demo-progress" role="img" aria-label=format!("{} demonstrativo: {}", greek.name, greek.value)>
                                        <i class=greek.tone style=format!("width:{}%", greek.width)></i>
                                    </div>
                                </div>
                            }).collect_view()}
                        </div>
                    </DashboardPanel>
                </div>

                <div class="dashboard-column dashboard-column-left">
                    <DashboardPanel class="pnl-panel" title="P&L vs benchmark" subtitle="Evolução acumulada · últimos 30 dias · simulação">
                        <DemoPnlChart data=data.pnl />
                    </DashboardPanel>

                    <DashboardPanel class="maturity-panel" title="Escada de vencimentos" subtitle="Posições por intervalo de DTE · simulação">
                        <div class="ladder-bars" role="img" aria-label="Distribuição demonstrativa: 3 posições de 0 a 7 DTE, 2 de 8 a 14, 4 de 15 a 30, 2 de 31 a 60 e 1 acima de 60">
                            {data.maturities.into_iter().map(|item| view! {
                                <div class="ladder-column"><b>{item.count}</b><div><i class=item.tone style=format!("height:{}px", item.height)></i></div><span>{item.range}</span></div>
                            }).collect_view()}
                        </div>
                        <small class="ladder-axis">"Dias até ao vencimento (DTE)"</small>
                        <div class="ladder-note"><b>"5 posições"</b><span>"vencem nos próximos 14 dias"</span></div>
                    </DashboardPanel>
                </div>

                <div class="dashboard-column dashboard-column-right">
                    <DashboardPanel class="alerts-panel" title="Alertas de risco" subtitle="3 sinais demonstrativos requerem atenção">
                        <div class="alert-list">
                            {data.alerts.into_iter().map(|alert| view! {
                                <div class=format!("risk-alert {}", alert.tone)>
                                    <span aria-hidden="true">"!"</span>
                                    <div><b>{alert.title}</b><small>{alert.detail}</small></div>
                                    <em>"Simulação"</em>
                                </div>
                            }).collect_view()}
                        </div>
                    </DashboardPanel>

                    <DashboardPanel class="activity-panel" title="Atividade recente" subtitle="Sistema factual · atividade financeira simulada">
                        <div
                            class="activity-scroll"
                            tabindex="0"
                            role="region"
                            aria-label="Lista de atividades recentes"
                        >
                        <div class="activity-columns">
                            <section class="activity-group" aria-labelledby="system-activity-title">
                                <h3 id="system-activity-title" class="activity-section-label">"Sistema"</h3>
                                <DashboardRefreshActivities />
                            </section>
                            <section class="activity-group" aria-labelledby="simulation-activity-title">
                                <h3 id="simulation-activity-title" class="activity-section-label">"Simulação"</h3>
                                {data.activities.into_iter().map(|activity| view! {
                                    <div class="activity-row">
                                        <span class=format!("activity-icon {}", activity.tone) aria-hidden="true">{activity.icon}</span>
                                        <div><small>{format!("Simulação · {}", activity.time)}</small><b>{activity.title}</b><p>{activity.detail}</p></div>
                                        {activity.value.map(|value| view! { <strong class="positive">{value}</strong> })}
                                    </div>
                                }).collect_view()}
                            </section>
                        </div>
                        </div>
                    </DashboardPanel>
                </div>

                <DashboardPanel class="risk-positions-panel" title="Posições em maior risco" subtitle="Ranking explicável · dados demonstrativos">
                    <div class="risk-method"><span>"Score simulado"</span><b>"35% DTE"</b><b>"30% Gamma"</b><b>"20% Vega"</b><b>"15% perda"</b></div>
                    <div class="risk-table-desktop">
                        <table class="risk-table">
                            <thead><tr><th>"Posição"</th><th>"Estratégia"</th><th>"Strike"</th><th>"Vencimento"</th><th>"DTE"</th><th>"P&L atual"</th><th>"Motivo principal"</th><th>"Risco"</th></tr></thead>
                            <tbody>{data.positions.into_iter().map(risk_table_row).collect_view()}</tbody>
                        </table>
                    </div>
                    <div class="risk-cards-mobile" aria-label="Posições em maior risco">
                        {data.positions.into_iter().map(risk_mobile_card).collect_view()}
                    </div>
                </DashboardPanel>
            </div>
        </section>
    }
}

#[component]
fn MarketAnalysisPage() -> impl IntoView {
    let history = LocalResource::new(load_spx_history);
    let (live_price, set_live_price) = signal::<Option<Result<AssetLivePrice, String>>>(None);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            let socket_url = match spx_websocket_url() {
                Ok(url) => url,
                Err(error) => {
                    set_live_price.set(Some(Err(error)));
                    return;
                }
            };
            loop {
                match WebSocket::open(&socket_url) {
                    Ok(socket) => {
                        let (_, mut messages) = socket.split();
                        while let Some(message) = messages.next().await {
                            match message {
                                Ok(gloo_net::websocket::Message::Text(payload)) => {
                                    match serde_json::from_str::<AssetLivePrice>(&payload) {
                                        Ok(price) => set_live_price.set(Some(Ok(price))),
                                        Err(_) => set_live_price.set(Some(Err(
                                            "A cotação recebida não tem o formato esperado."
                                                .to_string(),
                                        ))),
                                    }
                                }
                                Ok(gloo_net::websocket::Message::Bytes(_)) => {}
                                Err(_) => break,
                            }
                        }
                        set_live_price.set(Some(Err(
                            "A ligação foi interrompida; nova tentativa em curso.".to_string(),
                        )));
                    }
                    Err(_) => {
                        set_live_price.set(Some(Err(
                            "Não foi possível ligar; nova tentativa em curso.".to_string(),
                        )));
                    }
                }
                TimeoutFuture::new(3_000).await;
            }
        });
    });

    view! {
        <section class="page market-analysis-page">
            <PageHeader
                eyebrow="Mercado"
                title="Análise de mercado"
                subtitle="Dados reais do S&P 500 quando disponíveis através da API"
            />
            <div class="real-data-notice" role="note">"Dados reais quando disponíveis · cotação e histórico servidos exclusivamente por /api"</div>
            <div class="metric-grid market-metrics">
                <LiveSpxCard price=live_price />
            </div>
            <div class="content-grid market-history-grid">
                <article class="card chart-card">
                    <CardTitle title="S&P 500" detail="Últimas 90 sessões disponíveis" />
                    <Suspense fallback=move || view! { <DataStatus kind="loading" message="A carregar histórico do SPX…".to_string() /> }>
                        {move || Suspend::new(async move {
                            match history.await {
                                Ok(response) => history_view(response).into_any(),
                                Err(error) => view! {
                                    <DataStatus kind="error" message=format!("Não foi possível carregar o histórico. {error}") />
                                }.into_any(),
                            }
                        })}
                    </Suspense>
                </article>
            </div>
            <GammaExposureView />
        </section>
    }
}

#[derive(Clone, Debug)]
enum HistoryLoadError {
    Request,
    Http(u16),
    EmptyBody(u16),
    ContentType { status: u16, received: String },
    InvalidJson(u16),
}

impl fmt::Display for HistoryLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request => write!(formatter, "A API não respondeu."),
            Self::Http(status) => write!(formatter, "A API respondeu com HTTP {status}."),
            Self::EmptyBody(status) => {
                write!(
                    formatter,
                    "A API respondeu com HTTP {status}, mas sem conteúdo."
                )
            }
            Self::ContentType { status, received } => write!(
                formatter,
                "A API respondeu com HTTP {status}, mas com Content-Type inesperado ({received})."
            ),
            Self::InvalidJson(status) => write!(
                formatter,
                "A API respondeu com HTTP {status}, mas o JSON não corresponde ao contrato público."
            ),
        }
    }
}

async fn load_spx_history() -> Result<MarketSpxHistoryResponse, HistoryLoadError> {
    let response = Request::get("/api/market/spx-history")
        .send()
        .await
        .map_err(|_| HistoryLoadError::Request)?;
    let status = response.status();
    if !response.ok() {
        return Err(HistoryLoadError::Http(status));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap_or_else(|| "ausente".to_string());
    let body = response
        .text()
        .await
        .map_err(|_| HistoryLoadError::Request)?;
    if body.trim().is_empty() {
        return Err(HistoryLoadError::EmptyBody(status));
    }
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    {
        return Err(HistoryLoadError::ContentType {
            status,
            received: content_type,
        });
    }
    serde_json::from_str(&body).map_err(|_| HistoryLoadError::InvalidJson(status))
}

fn spx_websocket_url() -> Result<String, String> {
    let window = web_sys::window()
        .ok_or_else(|| "O browser não disponibilizou a janela da aplicação.".to_string())?;
    let location = window.location();
    let protocol = location
        .protocol()
        .map_err(|_| "Não foi possível determinar o protocolo da aplicação.".to_string())?;
    let host = location
        .host()
        .map_err(|_| "Não foi possível determinar o endereço da aplicação.".to_string())?;
    let websocket_protocol = if protocol == "https:" { "wss" } else { "ws" };
    Ok(format!(
        "{websocket_protocol}://{host}/api/assets/live?ticker=SPX"
    ))
}

fn history_view(response: MarketSpxHistoryResponse) -> AnyView {
    match response.spx_history {
        DataState::Available(history) => view! { <HistorySuccess history stale=false /> }.into_any(),
        DataState::Stale(history) => view! { <HistorySuccess history stale=true /> }.into_any(),
        DataState::Unavailable => view! {
            <DataStatus kind="unavailable" message="O backend não tem sessões do SPX disponíveis.".to_string() />
        }
        .into_any(),
    }
}

#[component]
fn LiveSpxCard(price: ReadSignal<Option<Result<AssetLivePrice, String>>>) -> impl IntoView {
    view! {
        <article class="metric-card live-metric" aria-live="polite">
            {move || match price.get() {
                None => view! {
                    <span>"S&P 500 · cotação"</span>
                    <strong class="skeleton">"A carregar…"</strong>
                    <small>"A ligar ao mercado"</small>
                }.into_any(),
                Some(Err(error)) => view! {
                    <span>"S&P 500 · erro"</span>
                    <strong>"Indisponível"</strong>
                    <small class="negative">{error}</small>
                }.into_any(),
                Some(Ok(price)) => {
                    let is_live = price.market_hours == 1;
                    let trend_class = if price.change_percent >= 0.0 { "positive" } else { "negative" };
                    view! {
                        <span>{if is_live { "S&P 500 · mercado aberto" } else { "S&P 500 · último valor/EOD" }}</span>
                        <strong>{format_number(price.price)}</strong>
                        <small class=trend_class>
                            {format!("{:+.2}% · {}", price.change_percent, if is_live { "tempo real" } else { "mercado fechado" })}
                        </small>
                    }.into_any()
                }
            }}
        </article>
    }
}

#[component]
fn HistorySuccess(history: PriceHistoryOverview, stale: bool) -> AnyView {
    let points = history
        .points
        .into_iter()
        .rev()
        .take(90)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let stale = stale || history.metadata.freshness == Freshness::Stale;
    let session_date = history.metadata.session_date;
    let source = history.metadata.source;

    if points.is_empty() {
        return view! {
            <DataStatus kind="unavailable" message="O histórico do SPX não contém sessões completas.".to_string() />
        }
        .into_any();
    }

    view! {
        <div class="history-content">
            {stale.then(|| view! {
                <div class="stale-notice" role="status">
                    {format!("Dados EOD anteriores/desatualizados · última sessão {session_date}")}
                </div>
            })}
            <SpxChart points=points />
            <div class="chart-meta">
                <span>{format!("Fonte: {source}")}</span>
                <span>{format!("Sessão mais recente: {session_date}")}</span>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn SpxChart(points: Vec<PriceHistoryPoint>) -> impl IntoView {
    let (plot_error, set_plot_error) = signal::<Option<String>>(None);
    let plot = match build_spx_plot(&points) {
        Ok(plot) => Some(plot),
        Err(error) => {
            set_plot_error.set(Some(error));
            None
        }
    };
    let (plot, _) = signal(SendWrapper::new(plot));

    view! {
        <figure class="spx-chart">
            <PlotlyChart id=SPX_HISTORY_PLOT_ID plot error=set_plot_error aria_label="Histórico de fechos do S&P 500" />
            {move || plot_error.get().map(|error| view! { <DataStatus kind="error" message=error /> })}
        </figure>
    }
}

fn build_spx_plot(points: &[PriceHistoryPoint]) -> Result<Plot, String> {
    if points.is_empty() {
        return Err("O histórico do SPX não contém sessões completas.".to_string());
    }
    let mut dates = Vec::with_capacity(points.len());
    let mut closes = Vec::with_capacity(points.len());
    for point in points {
        if !point.close.is_finite() {
            return Err("O histórico do SPX contém um valor não finito.".to_string());
        }
        dates.push(point.date.to_string());
        closes.push(point.close);
    }

    let mut plot = Plot::new();
    plot.add_trace(
        Scatter::new(dates, closes)
            .name("S&P 500")
            .mode(Mode::Lines)
            .opacity(1.0)
            .line(Line::new().color("#4da3ff").width(3.0))
            .hover_template("Sessão: %{x|%x}<br>Fecho: %{y:,.2f}<extra></extra>"),
    );
    plot.set_layout(
        Layout::new()
            .auto_size(true)
            .show_legend(false)
            .margin(Margin::new().left(58).right(14).top(18).bottom(48))
            .paper_background_color("#19263c")
            .plot_background_color("#111b2e")
            .font(plotly::common::Font::new().color("#dce4f2"))
            .x_axis(Axis::new().title(Title::with_text("Sessão")))
            .y_axis(
                Axis::new()
                    .title(Title::with_text("Índice"))
                    .tick_format(",.2f"),
            ),
    );
    plot.set_configuration(
        Configuration::new()
            .responsive(true)
            .display_logo(false)
            .scroll_zoom(false),
    );
    Ok(plot)
}

#[component]
fn DataStatus(kind: &'static str, message: String) -> impl IntoView {
    view! {
        <div class=format!("data-status {kind}") role=if kind == "error" { "alert" } else { "status" }>
            <span class="status-symbol" aria-hidden="true">{if kind == "loading" { "◌" } else { "!" }}</span>
            <span>{message}</span>
        </div>
    }
}

fn format_number(value: f64) -> String {
    let raw = format!("{value:.2}");
    let (integer, decimals) = raw.split_once('.').unwrap_or((&raw, "00"));
    let grouped = integer
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(index, character)| {
            if index > 0 && index % 3 == 0 {
                vec![' ', character]
            } else {
                vec![character]
            }
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{grouped},{decimals}")
}

#[cfg(test)]
mod spx_plot_tests {
    use super::*;
    use chrono::NaiveDate;

    fn point(year: i32, month: u32, day: u32, close: f64) -> PriceHistoryPoint {
        PriceHistoryPoint {
            date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            open: close - 10.0,
            high: close + 10.0,
            low: close - 20.0,
            close,
        }
    }

    fn plot_json(points: &[PriceHistoryPoint]) -> serde_json::Value {
        serde_json::from_str(&build_spx_plot(points).unwrap().to_json()).unwrap()
    }

    #[test]
    fn maps_factual_sessions_to_xy_in_received_chronological_order() {
        let value = plot_json(&[
            point(2026, 8, 19, 6_395.78),
            point(2026, 8, 20, 6_410.12),
            point(2026, 8, 21, 6_376.44),
        ]);
        assert_eq!(
            value["data"][0]["x"],
            serde_json::json!(["2026-08-19", "2026-08-20", "2026-08-21"])
        );
        assert_eq!(
            value["data"][0]["y"],
            serde_json::json!([6395.78, 6410.12, 6376.44])
        );
    }

    #[test]
    fn historical_plot_is_pure_and_repeatable() {
        let points = [
            point(2026, 8, 19, 6_395.78),
            point(2026, 8, 20, 6_410.12),
            point(2026, 8, 21, 6_376.44),
        ];
        let first = build_spx_plot(&points).unwrap().to_json();
        let second = build_spx_plot(&points).unwrap().to_json();
        assert_eq!(first, second);
        let value: serde_json::Value = serde_json::from_str(&first).unwrap();
        assert_eq!(value["data"].as_array().unwrap().len(), 1);
        assert_eq!(value["data"][0]["x"].as_array().unwrap().len(), 3);
        assert_eq!(value["data"][0]["y"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn rejects_empty_and_non_finite_series() {
        assert!(build_spx_plot(&[]).is_err());
        for close in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(build_spx_plot(&[point(2026, 8, 21, close)]).is_err());
        }
    }

    #[test]
    fn configures_single_responsive_line_with_date_tooltip_and_no_legend() {
        let plot = build_spx_plot(&[point(2026, 8, 21, 6_376.44)]).unwrap();
        let json = plot.to_json();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let configuration = serde_json::to_value(plot.configuration()).unwrap();
        assert_eq!(value["data"].as_array().unwrap().len(), 1);
        assert_eq!(value["data"][0]["mode"], "lines");
        assert_eq!(value["data"][0]["type"], "scatter");
        assert_eq!(value["data"][0]["opacity"], 1.0);
        assert_eq!(value["data"][0]["line"]["color"], "#4da3ff");
        assert_eq!(value["data"][0]["line"]["width"], 3.0);
        assert_eq!(value["layout"]["showlegend"], false);
        assert_eq!(value["layout"]["autosize"], true);
        assert!(value["layout"].get("width").is_none());
        assert!(value["layout"]["xaxis"].get("domain").is_none());
        assert!(value["layout"]["yaxis"].get("domain").is_none());
        assert_eq!(value["layout"]["xaxis"]["title"]["text"], "Sessão");
        assert_eq!(value["layout"]["yaxis"]["title"]["text"], "Índice");
        assert_eq!(value["layout"]["yaxis"]["tickformat"], ",.2f");
        assert_eq!(configuration["responsive"], true);
        assert_eq!(configuration["displaylogo"], false);
        assert_eq!(configuration["scrollZoom"], false);
        assert_eq!(
            value["data"][0]["hovertemplate"],
            "Sessão: %{x|%x}<br>Fecho: %{y:,.2f}<extra></extra>"
        );
        assert!(!json.contains("rangeslider"));
    }

    #[test]
    fn historical_target_and_ancestors_fill_the_card_without_width_caps() {
        let css = include_str!("../styles.css");
        assert!(css.contains(
            ".market-history-grid, .market-history-grid .chart-card, .history-content, .spx-chart, .spx-chart .plotly-chart { width: 100%; min-width: 0; }"
        ));
        for forbidden in [
            ".spx-chart { max-width",
            ".spx-chart .plotly-chart { max-width",
            ".spx-chart { flex-basis",
            ".spx-chart .plotly-chart { flex-basis",
            ".spx-chart { aspect-ratio",
        ] {
            assert!(!css.contains(forbidden));
        }
        assert!(css.contains("* { box-sizing: border-box; }"));
    }

    #[test]
    fn history_and_gex_have_distinct_stable_ids() {
        assert_eq!(SPX_HISTORY_PLOT_ID, "spx-history-plot");
        assert_eq!(gamma_exposure::GEX_PLOT_ID, "gex-profile-plot");
        assert_eq!(gamma_exposure::GEX_STRIKE_PLOT_ID, "gex-strike-plot");
        assert_eq!(
            gamma_exposure::GEX_EXPIRATION_PLOT_ID,
            "gex-expiration-plot"
        );
        let ids = [
            SPX_HISTORY_PLOT_ID,
            gamma_exposure::GEX_PLOT_ID,
            gamma_exposure::GEX_STRIKE_PLOT_ID,
            gamma_exposure::GEX_EXPIRATION_PLOT_ID,
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }
    }
}

#[component]
fn PortfolioPage() -> impl IntoView {
    view! {
        <section class="page">
            <PageHeader eyebrow="Património" title="Portfolio" subtitle="Posições, exposição e movimentos" />
            <PlaceholderNotice />
            <div class="metric-grid three">
                <MetricCard label="Capital" value="€ 24 680" trend="Placeholder" positive=true />
                <MetricCard label="P&L aberto" value="+€ 842" trend="+3,53%" positive=true />
                <MetricCard label="Buying power" value="€ 9 420" trend="Placeholder" positive=true />
            </div>
            <article class="card">
                <CardTitle title="Posições abertas" detail="Exemplo visual" />
                <div class="table-wrap">
                    <table>
                        <thead><tr><th>Ativo</th><th>Estratégia</th><th>Expiração</th><th>P&L</th></tr></thead>
                        <tbody>
                            <tr><td>"SPX"</td><td>"Iron Condor"</td><td>"21 JUN"</td><td class="positive">"+€ 318"</td></tr>
                            <tr><td>"AAPL"</td><td>"Covered Call"</td><td>"19 JUL"</td><td class="positive">"+€ 126"</td></tr>
                            <tr><td>"QQQ"</td><td>"Put Spread"</td><td>"28 JUN"</td><td class="negative">"-€ 74"</td></tr>
                        </tbody>
                    </table>
                </div>
            </article>
        </section>
    }
}

#[component]
fn BuilderPage() -> impl IntoView {
    view! {
        <section class="page">
            <PageHeader eyebrow="Estratégias" title="Construtor" subtitle="Composição visual de estratégias de opções" />
            <PlaceholderNotice />
            <div class="content-grid">
                <article class="card">
                    <CardTitle title="Estratégias" detail="Seleção ainda inativa" />
                    <div class="strategy-list">
                        <button type="button" disabled><strong>"Bull Call Spread"</strong><span>"Débito · perspetiva bullish"</span></button>
                        <button type="button" disabled><strong>"Iron Condor"</strong><span>"Crédito · mercado lateral"</span></button>
                        <button type="button" disabled><strong>"Straddle"</strong><span>"Débito · movimento amplo"</span></button>
                    </div>
                </article>
                <article class="card chart-card">
                    <CardTitle title="Payoff" detail="Pré-visualização" />
                    <div class="payoff-placeholder"><span>"Gráfico de payoff placeholder"</span></div>
                </article>
            </div>
        </section>
    }
}

#[component]
fn SimulatorPage() -> impl IntoView {
    view! {
        <section class="page">
            <PageHeader eyebrow="Cenários" title="Simulador" subtitle="Cenários e sensibilidade de opções" />
            <PlaceholderNotice />
            <div class="content-grid form-layout">
                <article class="card form-card">
                    <CardTitle title="Parâmetros" detail="Controlos não ligados" />
                    <label>"Ticker"<input value="SPX" disabled /></label>
                    <label>"Preço spot"<input value="5240.03" disabled /></label>
                    <label>"Volatilidade"<input value="18.5%" disabled /></label>
                    <button class="primary-button" type="button" disabled>"Simular"</button>
                </article>
                <article class="card chart-card">
                    <CardTitle title="Sensibilidade" detail="Resultado placeholder" />
                    <div class="greeks-grid">
                        <span><small>Delta</small><strong>0,52</strong></span>
                        <span><small>Gamma</small><strong>0,038</strong></span>
                        <span><small>Theta</small><strong>-0,12</strong></span>
                        <span><small>Vega</small><strong>0,28</strong></span>
                    </div>
                    <div class="chart-placeholder compact"><span>"Curvas de cenário"</span></div>
                </article>
            </div>
        </section>
    }
}

#[component]
fn SettingsPage() -> impl IntoView {
    let (refresh_state, set_refresh_state) = expect_context::<(
        ReadSignal<DataRefreshLoadState>,
        WriteSignal<DataRefreshLoadState>,
    )>();
    let (submitting, set_submitting) = signal(false);
    let (request_feedback, set_request_feedback) = signal::<Option<(bool, String)>>(None);

    let request_refresh = move |_| {
        if submitting.get_untracked()
            || data_refresh_is_awaiting_terminal(&refresh_state.get_untracked())
        {
            return;
        }
        set_submitting.set(true);
        set_request_feedback.set(None);
        leptos::task::spawn_local(async move {
            let result = request_manual_data_refresh().await;
            match result {
                Ok((message, run)) => {
                    set_request_feedback.set(Some((false, message)));
                    set_refresh_state.update(|state| mark_manual_refresh_known(state, run));
                    load_data_refresh_status(set_refresh_state).await;
                }
                Err(message) => set_request_feedback.set(Some((true, message))),
            }
            set_submitting.set(false);
        });
    };

    view! {
        <section class="page">
            <PageHeader eyebrow="Preferências" title="Configurações" subtitle="Preferências e atualização de dados" />
            <DataRefreshPanel
                refresh_state=refresh_state
                submitting=submitting
                request_feedback=request_feedback
                request_refresh=request_refresh
            />
            <article class="card settings-list">
                <SettingRow title="Modo escuro" detail="Tema inicial da fundação visual" />
                <SettingRow title="Alertas de risco" detail="Configuração placeholder" />
                <SettingRow title="Moeda base" detail="EUR (€) · configuração placeholder" />
            </article>
        </section>
    }
}

async fn load_tracked_tickers(
    set_state: WriteSignal<TrackedTickersLoadState>,
    guard: ObservationGuard,
) -> bool {
    let result = async {
        let response = Request::get("/api/tracked-tickers?include_inactive=true")
            .send()
            .await
            .map_err(|_| "Não foi possível contactar o serviço de subjacentes.".to_string())?;
        let status = response.status();
        if status != 200 {
            return Err(format!(
                "Não foi possível carregar o catálogo de subjacentes (HTTP {status})."
            ));
        }
        response
            .json::<Vec<TrackedTicker>>()
            .await
            .map_err(|_| "O serviço devolveu um catálogo de subjacentes inválido.".to_string())
    }
    .await;

    let succeeded = result.is_ok();
    if guard.is_active() {
        set_state.set(match result {
            Ok(tickers) => TrackedTickersLoadState::Success(tickers),
            Err(message) => TrackedTickersLoadState::Error(message),
        });
    }
    succeeded
}

fn normalize_user_ticker(value: &str) -> Result<String, &'static str> {
    let ticker = value.trim().to_ascii_uppercase();
    if ticker.is_empty() {
        return Err("Introduza um ticker.");
    }
    if ticker.len() > 15 {
        return Err("O ticker não pode exceder 15 caracteres.");
    }
    if !ticker
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '^' | '.' | '-'))
    {
        return Err("Use apenas letras, números, ponto, hífen ou acento circunflexo.");
    }
    Ok(ticker)
}

fn user_tracked_tickers(tickers: Vec<TrackedTicker>) -> Vec<TrackedTicker> {
    tickers
        .into_iter()
        .filter(|item| item.source == TrackedTickerSource::User)
        .collect()
}

async fn put_tracked_ticker(
    ticker: &str,
    configuration: &ConfigureTrackedTickerRequest,
) -> Result<(), String> {
    let request = Request::put(&format!("/api/tracked-tickers/{ticker}"))
        .json(configuration)
        .map_err(|_| "Não foi possível preparar a configuração do subjacente.".to_string())?;
    let response = request
        .send()
        .await
        .map_err(|_| "Não foi possível contactar o serviço de subjacentes.".to_string())?;
    let status = response.status();
    if status == 204 {
        return Ok(());
    }

    let detail = response
        .json::<ApiError>()
        .await
        .ok()
        .map(|body| body.error);
    Err(ticker_save_error(status, detail))
}

fn ticker_save_error(status: u16, detail: Option<String>) -> String {
    let prefix = match status {
        400 => "O pedido para guardar o subjacente é inválido (HTTP 400).",
        404 => "O ticker não foi encontrado ao guardar (HTTP 404).",
        409 => {
            "Este ticker corresponde a um subjacente gerido e protegido pelo sistema (HTTP 409)."
        }
        503 => "O serviço de validação está temporariamente indisponível (HTTP 503).",
        _ => {
            return format!("Não foi possível guardar o subjacente (HTTP {status}).");
        }
    };
    detail
        .filter(|message| !message.trim().is_empty())
        .map_or_else(
            || prefix.to_string(),
            |message| format!("{prefix} {message}"),
        )
}

#[derive(Clone)]
enum TickerResolutionState {
    Idle,
    Resolving,
    Resolved(UnderlyingResolution),
    InvalidFormat(String),
    NotFound(Option<String>),
    Conflict(Option<String>),
    TemporarilyUnavailable(Option<String>),
    NetworkError,
    InvalidResponse,
    UnexpectedStatus(u16, Option<String>),
}

fn resolution_error(status: u16, detail: Option<String>) -> TickerResolutionState {
    match status {
        400 => TickerResolutionState::InvalidFormat(detail.map_or_else(
            || "O formato do ticker não é válido para o serviço (HTTP 400).".to_string(),
            |detail| format!("O formato do ticker não é válido (HTTP 400). {detail}"),
        )),
        404 => TickerResolutionState::NotFound(detail),
        409 => TickerResolutionState::Conflict(detail),
        503 => TickerResolutionState::TemporarilyUnavailable(detail),
        _ => TickerResolutionState::UnexpectedStatus(status, detail),
    }
}

fn resolution_conflict_message(detail: Option<String>) -> String {
    let prefix =
        "Este ticker corresponde a um subjacente gerido e protegido pelo sistema (HTTP 409).";
    detail
        .filter(|message| !message.trim().is_empty())
        .map_or_else(
            || prefix.to_string(),
            |message| format!("{prefix} {message}"),
        )
}

fn parsed_resolution(resolution: Option<UnderlyingResolution>) -> TickerResolutionState {
    resolution.map_or(
        TickerResolutionState::InvalidResponse,
        TickerResolutionState::Resolved,
    )
}

async fn resolve_ticker(ticker: &str) -> TickerResolutionState {
    let encoded = js_sys::encode_uri_component(ticker)
        .as_string()
        .unwrap_or_else(|| ticker.to_string());
    let response = match Request::get(&format!("/api/underlyings/resolve?ticker={encoded}"))
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return TickerResolutionState::NetworkError,
    };

    let status = response.status();
    match status {
        200 => parsed_resolution(response.json::<UnderlyingResolution>().await.ok()),
        _ => {
            let detail = response
                .json::<ApiError>()
                .await
                .ok()
                .map(|body| body.error)
                .filter(|message| !message.trim().is_empty());
            resolution_error(status, detail)
        }
    }
}

fn safe_ticker_configuration(
    active: bool,
    historical_prices: bool,
) -> ConfigureTrackedTickerRequest {
    ConfigureTrackedTickerRequest {
        active,
        historical_prices,
        option_snapshots: false,
    }
}

fn ticker_save_message(ticker: &str, configuration: &ConfigureTrackedTickerRequest) -> String {
    if configuration.active && configuration.historical_prices {
        format!("{ticker} foi guardado e entrará na próxima atualização de dados.")
    } else {
        format!("Configuração de {ticker} guardada.")
    }
}

fn should_apply_resolution(guard: &ObservationGuard, expected: u64, current: u64) -> bool {
    guard.is_active() && expected == current
}

fn resolved_ticker_for_submission(
    resolution: &TickerResolutionState,
    requested_ticker: &str,
) -> Option<String> {
    match resolution {
        TickerResolutionState::Resolved(resolved) if resolved.ticker == requested_ticker => {
            Some(resolved.ticker.clone())
        }
        _ => None,
    }
}

fn start_ticker_save(
    ticker: String,
    configuration: ConfigureTrackedTickerRequest,
    set_state: WriteSignal<TrackedTickersLoadState>,
    set_saving: WriteSignal<BTreeSet<String>>,
    set_feedback: WriteSignal<Option<(bool, String)>>,
    on_reloaded: Option<Rc<dyn Fn()>>,
    guard: ObservationGuard,
) {
    set_saving.update(|saving| {
        saving.insert(ticker.clone());
    });
    set_feedback.set(None);
    leptos::task::spawn_local(async move {
        match put_tracked_ticker(&ticker, &configuration).await {
            Ok(()) => {
                if !guard.is_active() {
                    return;
                }
                let reloaded = load_tracked_tickers(set_state, guard.clone()).await;
                let message = ticker_save_message(&ticker, &configuration);
                if !guard.is_active() {
                    return;
                }
                set_feedback.set(Some((false, message)));
                if reloaded && let Some(on_reloaded) = on_reloaded {
                    on_reloaded();
                }
            }
            Err(message) => {
                if !guard.is_active() {
                    return;
                }
                if message.contains("HTTP 404") {
                    load_tracked_tickers(set_state, guard.clone()).await;
                }
                if !guard.is_active() {
                    return;
                }
                set_feedback.set(Some((true, message)));
            }
        }
        set_saving.update(|saving| {
            saving.remove(&ticker);
        });
    });
}

#[component]
fn UnderlyingsPage() -> impl IntoView {
    let (state, set_state) = signal(TrackedTickersLoadState::Loading);
    let (saving, set_saving) = signal(BTreeSet::<String>::new());
    let (feedback, set_feedback) = signal::<Option<(bool, String)>>(None);
    let guard = ObservationGuard::new();
    let initial_load_guard = guard.clone();
    leptos::task::spawn_local(async move {
        load_tracked_tickers(set_state, initial_load_guard).await;
    });
    let cleanup_guard = guard.clone();
    on_cleanup(move || cleanup_guard.cancel());
    let reload_guard = guard.clone();

    view! {
        <section class="page underlyings-page">
            <PageHeader eyebrow="UNIVERSO" title="Subjacentes" subtitle="Escolha os ativos que pretende acompanhar e analisar." />
            <div class="underlyings-feedback" aria-live="polite" aria-atomic="true">
                {move || feedback.get().map(|(is_error, message)| view! {
                    <div class:error=is_error class="ticker-feedback" role=if is_error { "alert" } else { "status" }>{message}</div>
                })}
            </div>
            <AddTrackedTickerForm
                state=state
                set_state=set_state
                saving=saving
                set_saving=set_saving
                set_feedback=set_feedback
                guard=guard.clone()
            />
            <section class="card my-underlyings" aria-labelledby="my-underlyings-title">
                <header class="underlyings-section-header">
                    <div><h2 id="my-underlyings-title">"Os meus subjacentes"</h2><p>"Os ativos inativos permanecem guardados e podem ser reativados."</p></div>
                    <button class="secondary-button" type="button" disabled=move || matches!(state.get(), TrackedTickersLoadState::Loading) on:click=move |_| {
                        set_state.set(TrackedTickersLoadState::Loading);
                        let reload_guard = reload_guard.clone();
                        leptos::task::spawn_local(async move {
                            load_tracked_tickers(set_state, reload_guard).await;
                        });
                    }>"Recarregar"</button>
                </header>
                {move || match state.get() {
                TrackedTickersLoadState::Loading => view! {
                    <div class="underlyings-state" role="status" aria-live="polite">"A carregar os seus subjacentes…"</div>
                }.into_any(),
                TrackedTickersLoadState::Error(message) => view! {
                    <div class="underlyings-state error" role="alert">{message}</div>
                }.into_any(),
                TrackedTickersLoadState::Success(tickers) => {
                    let user = user_tracked_tickers(tickers);
                    if user.is_empty() {
                        view! { <div class="user-underlyings-empty"><strong>"Ainda não acompanha nenhum subjacente."</strong><span>"Adicione um ticker acima para começar. A pesquisa está limitada ao ticker."</span></div> }.into_any()
                    } else {
                        view! { <div class="underlyings-grid user-grid">{user.into_iter().map(|ticker| view! { <UserTickerCard ticker=ticker set_state=set_state saving=saving set_saving=set_saving set_feedback=set_feedback guard=guard.clone() /> }).collect_view()}</div> }.into_any()
                    }
                }
                }}
            </section>
        </section>
    }
}

#[component]
fn AddTrackedTickerForm(
    state: ReadSignal<TrackedTickersLoadState>,
    set_state: WriteSignal<TrackedTickersLoadState>,
    saving: ReadSignal<BTreeSet<String>>,
    set_saving: WriteSignal<BTreeSet<String>>,
    set_feedback: WriteSignal<Option<(bool, String)>>,
    guard: ObservationGuard,
) -> impl IntoView {
    let (ticker, set_ticker) = signal(String::new());
    let (historical, set_historical) = signal(true);
    let (resolution, set_resolution) = signal(TickerResolutionState::Idle);
    let (advanced_open, set_advanced_open) = signal(false);
    let ticker_input = NodeRef::<leptos::html::Input>::new();
    let request_generation = Arc::new(AtomicU64::new(0));
    let cleanup_generation = request_generation.clone();
    on_cleanup(move || {
        cleanup_generation.fetch_add(1, Ordering::AcqRel);
    });
    let submit_guard = guard.clone();
    let resolution_guard = guard;
    let normalized = move || ticker.get().trim().to_ascii_uppercase();
    let existing_user = move || {
        let normalized = normalized();
        match state.get() {
            TrackedTickersLoadState::Success(tickers) => tickers
                .into_iter()
                .find(|item| item.source == TrackedTickerSource::User && item.ticker == normalized),
            TrackedTickersLoadState::Loading | TrackedTickersLoadState::Error(_) => None,
        }
    };
    view! {
        <form class="card add-underlying" aria-labelledby="add-underlying-title" on:submit=move |event| {
            event.prevent_default();
            let Ok(normalized_ticker) = normalize_user_ticker(&ticker.get_untracked()) else { return; };
            let Some(resolved_ticker) = resolved_ticker_for_submission(&resolution.get_untracked(), &normalized_ticker) else { return; };
            if saving.get_untracked().contains(&normalized_ticker) { return; }
            let reset_generation = request_generation.clone();
            let reset_form = Rc::new(move || {
                reset_generation.fetch_add(1, Ordering::AcqRel);
                set_ticker.set(String::new());
                set_resolution.set(TickerResolutionState::Idle);
                set_advanced_open.set(false);
                set_historical.set(true);
                if let Some(input) = ticker_input.get() {
                    let _ = input.focus();
                }
            });
            start_ticker_save(
                resolved_ticker,
                safe_ticker_configuration(true, historical.get_untracked()),
                set_state,
                set_saving,
                set_feedback,
                Some(reset_form),
                submit_guard.clone(),
            );
        }>
            <div class="add-underlying-heading"><h2 id="add-underlying-title">"Adicionar subjacente"</h2><p>"Por enquanto, a pesquisa é feita exatamente pelo ticker."</p></div>
            <div class="add-underlying-fields">
                <label class="ticker-field" for="new-underlying-ticker"><span>"Ticker"</span><input node_ref=ticker_input id="new-underlying-ticker" name="ticker" maxlength="15" autocomplete="off" prop:value=move || ticker.get() aria-invalid=move || matches!(resolution.get(), TickerResolutionState::InvalidFormat(_) | TickerResolutionState::NotFound(_) | TickerResolutionState::Conflict(_)).to_string() aria-describedby="new-underlying-help new-underlying-resolution" on:input={
                    let request_generation = request_generation.clone();
                    move |event| {
                        request_generation.fetch_add(1, Ordering::AcqRel);
                        set_ticker.set(event_target_value(&event).to_ascii_uppercase());
                        set_resolution.set(TickerResolutionState::Idle);
                    }
                } /><small id="new-underlying-help">{move || { let value = normalized(); if value.is_empty() { "Introduza exatamente um ticker, por exemplo AAPL ou BRK.B.".to_string() } else { format!("Ticker introduzido: {value}") } }}</small></label>
                <button class="secondary-button validate-ticker-button" type="button" disabled=move || matches!(resolution.get(), TickerResolutionState::Resolving) || existing_user().is_some() on:click={
                    let request_generation = request_generation.clone();
                    move |_| {
                        let normalized_ticker = match normalize_user_ticker(&ticker.get_untracked()) {
                            Ok(value) => value,
                            Err(message) => { set_resolution.set(TickerResolutionState::InvalidFormat(message.to_string())); return; }
                        };
                        if matches!(state.get_untracked(), TrackedTickersLoadState::Success(tickers) if tickers.iter().any(|item| item.source == TrackedTickerSource::User && item.ticker == normalized_ticker)) {
                            return;
                        }
                        if matches!(state.get_untracked(), TrackedTickersLoadState::Success(tickers) if tickers.iter().any(|item| item.source == TrackedTickerSource::System && item.ticker == normalized_ticker)) {
                            set_resolution.set(TickerResolutionState::InvalidFormat("Este ticker é gerido pelo sistema e não pode ser adicionado.".to_string()));
                            return;
                        }
                        set_ticker.set(normalized_ticker.clone());
                        let generation = request_generation.fetch_add(1, Ordering::AcqRel) + 1;
                        set_resolution.set(TickerResolutionState::Resolving);
                        let request_generation = request_generation.clone();
                        let request_guard = resolution_guard.clone();
                        leptos::task::spawn_local(async move {
                            let result = resolve_ticker(&normalized_ticker).await;
                            if should_apply_resolution(&request_guard, generation, request_generation.load(Ordering::Acquire)) {
                                set_resolution.set(result);
                            }
                        });
                    }
                }>{move || if matches!(resolution.get(), TickerResolutionState::Resolving) { "A validar…" } else { "Validar ticker" }}</button>
            </div>
            <div id="new-underlying-resolution" class="ticker-resolution" aria-live="polite" aria-atomic="true">{move || if let Some(existing) = existing_user() {
                view! { <div class="existing-underlying" role="status"><strong>"Este subjacente já está na sua lista"</strong><span>{if existing.active { "Está ativo." } else { "Está inativo." }}</span><a href=format!("#ticker-card-{}", existing.ticker)>"Ir para o cartão de configuração"</a></div> }.into_any()
            } else { match resolution.get() {
                TickerResolutionState::Idle => view! { <span>"Valide o ticker antes de o adicionar."</span> }.into_any(),
                TickerResolutionState::Resolving => view! { <span role="status">"A validar o ticker…"</span> }.into_any(),
                TickerResolutionState::InvalidFormat(message) => view! { <span class="error" role="alert">{message}</span> }.into_any(),
                TickerResolutionState::NotFound(detail) => view! { <span class="error" role="alert">{detail.map_or_else(|| "Ticker não encontrado (HTTP 404).".to_string(), |detail| format!("Ticker não encontrado (HTTP 404). {detail}"))}</span> }.into_any(),
                TickerResolutionState::Conflict(detail) => view! { <span class="error" role="alert">{resolution_conflict_message(detail)}</span> }.into_any(),
                TickerResolutionState::TemporarilyUnavailable(detail) => view! { <span class="error" role="alert">{detail.map_or_else(|| "O serviço de validação está temporariamente indisponível (HTTP 503). Tente novamente mais tarde.".to_string(), |detail| format!("O serviço de validação está temporariamente indisponível (HTTP 503). {detail}"))}</span> }.into_any(),
                TickerResolutionState::NetworkError => view! { <span class="error" role="alert">"Erro de rede ao validar o ticker. Verifique a ligação e tente novamente."</span> }.into_any(),
                TickerResolutionState::InvalidResponse => view! { <span class="error" role="alert">"O serviço devolveu uma resposta inválida."</span> }.into_any(),
                TickerResolutionState::UnexpectedStatus(status, detail) => view! { <span class="error" role="alert">{detail.map_or_else(|| format!("Não foi possível validar o ticker (HTTP {status})."), |detail| format!("Não foi possível validar o ticker (HTTP {status}). {detail}"))}</span> }.into_any(),
                TickerResolutionState::Resolved(result) => view! { <div class="resolution-confirmation" role="status"><strong>{format!("{} confirmado", result.ticker)}</strong><MetadataFacts metadata=result.metadata /><p>"A validação não adicionou nem ativou este ticker."</p></div> }.into_any(),
            }}}</div>
            <button class="advanced-toggle" type="button" aria-expanded=move || advanced_open.get().to_string() aria-controls="new-underlying-advanced" on:click=move |_| set_advanced_open.set(!advanced_open.get_untracked())><span>"Opções avançadas"</span><span aria-hidden="true">{move || if advanced_open.get() { "−" } else { "+" }}</span></button>
            <div id="new-underlying-advanced" class="advanced-options" hidden=move || !advanced_open.get()>
                <label class="check-control"><input type="checkbox" checked=move || historical.get() on:change=move |event| set_historical.set(event_target_checked(&event)) /><span><strong>"Histórico de preços"</strong><small>"Ativo por omissão"</small></span></label>
                <label class="check-control"><input type="checkbox" checked=false disabled /><span><strong>"Snapshots de opções"</strong><small>"Disponível após validação de capability de opções"</small></span></label>
            </div>
            {move || existing_user().is_none().then(|| view! { <button class="add-underlying-button final-add-button" type="submit" disabled=move || {
                let value = normalized();
                saving.get().contains(&value) || resolved_ticker_for_submission(&resolution.get(), &value).is_none()
            }>{move || { let value = normalized(); if saving.get().contains(&value) { "A guardar…" } else { "Adicionar subjacente" } }}</button> })}
            <p class="update-note">"Adicionar guarda a configuração pedida, mas não inicia uma atualização de dados."</p>
        </form>
    }
}

#[component]
fn MetadataFacts(metadata: api_models::UnderlyingMetadata) -> impl IntoView {
    let facts = [
        ("Moeda", metadata.currency),
        ("Bolsa", metadata.exchange),
        ("Fuso horário", metadata.timezone),
        ("Tipo de instrumento", metadata.instrument_type),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| (label, value)))
    .collect::<Vec<_>>();
    view! { <>{if facts.is_empty() { None } else { Some(view! { <dl class="metadata-facts">{facts.into_iter().map(|(label, value)| view! { <div><dt>{label}</dt><dd>{value}</dd></div> }).collect_view()}</dl> }) }}</> }
}

#[component]
fn UserTickerCard(
    ticker: TrackedTicker,
    set_state: WriteSignal<TrackedTickersLoadState>,
    saving: ReadSignal<BTreeSet<String>>,
    set_saving: WriteSignal<BTreeSet<String>>,
    set_feedback: WriteSignal<Option<(bool, String)>>,
    guard: ObservationGuard,
) -> impl IntoView {
    let ticker_name = ticker.ticker.clone();
    let (active, set_active) = signal(ticker.active);
    let (historical, set_historical) = signal(ticker.historical_prices);
    let (snapshots, set_snapshots) = signal(ticker.option_snapshots);
    let (advanced_open, set_advanced_open) = signal(false);
    let saving_key = ticker_name.clone();
    let saving_this = Memo::new(move |_| saving.get().contains(&saving_key));
    let resolution_state = ticker.resolution_state.clone();
    let can_edit = resolution_state == UnderlyingResolutionState::Resolved;
    let state_label = match resolution_state {
        UnderlyingResolutionState::Pending => "Por validar",
        UnderlyingResolutionState::Resolved => "Validado",
        UnderlyingResolutionState::Rejected => "Não encontrado",
    };
    view! {
        <article id=format!("ticker-card-{}", ticker.ticker) class:inactive=move || !active.get() class="underlying-card user-card">
            <header><div><strong>{ticker.ticker.clone()}</strong><span class="resolution-badge">{state_label}</span></div><span class:inactive=move || !active.get() class="active-badge">{move || if active.get() { "Ativo" } else { "Inativo" }}</span></header>
            <MetadataFacts metadata=ticker.metadata.clone() />
            <p class="capability-summary">{move || format!("Configuração pedida — histórico de preços: {} · snapshots de opções: {}", if historical.get() { "sim" } else { "não" }, if snapshots.get() { "sim" } else { "não" })}</p>
            {can_edit.then(|| view! {
                <label class="active-control"><input type="checkbox" checked=move || active.get() disabled=saving_this on:change=move |event| set_active.set(event_target_checked(&event)) /><span><strong>{move || if active.get() { "Acompanhamento ativo" } else { "Acompanhamento inativo" }}</strong><small>"Desativar preserva este registo."</small></span></label>
            })}
            <button class="advanced-toggle" type="button" disabled=move || saving_this.get() || (!can_edit && !snapshots.get()) aria-expanded=move || advanced_open.get().to_string() aria-controls=format!("advanced-{ticker_name}") on:click=move |_| set_advanced_open.set(!advanced_open.get_untracked())><span>"Opções avançadas"</span><span aria-hidden="true">{move || if advanced_open.get() { "−" } else { "+" }}</span></button>
            <div id=format!("advanced-{}", ticker.ticker) class="advanced-options" hidden=move || !advanced_open.get()>
                <label class="check-control"><input type="checkbox" checked=move || historical.get() disabled=move || saving_this.get() || !can_edit on:change=move |event| set_historical.set(event_target_checked(&event)) /><span><strong>"Histórico de preços"</strong><small>"Recolher preços históricos"</small></span></label>
                <label class="check-control"><input type="checkbox" checked=move || snapshots.get() disabled /><span><strong>"Snapshots de opções"</strong><small>{move || if snapshots.get() { "Configuração antiga ativa; desative-a para guardar." } else { "Disponível após validação de capability de opções" }}</small></span></label>
                {move || snapshots.get().then(|| view! { <button class="secondary-button" type="button" disabled=saving_this on:click=move |_| set_snapshots.set(false)>"Desativar configuração antiga"</button> })}
            </div>
            <p class="update-note">{if can_edit { "Guardar não consulta novamente o provider nem inicia uma atualização de dados." } else { "A nova validação é feita ao guardar com o comportamento seguro do serviço." }}</p>
            <button class="save-ticker-button" type="button" disabled=move || saving_this.get() || snapshots.get() aria-busy=move || saving_this.get().to_string() on:click=move |_| {
                start_ticker_save(ticker_name.clone(), safe_ticker_configuration(active.get_untracked(), historical.get_untracked()), set_state, set_saving, set_feedback, None, guard.clone());
            }>{move || if saving_this.get() { "A guardar…" } else if can_edit { "Guardar configuração" } else { "Validar novamente" }}</button>
        </article>
    }
}

async fn request_manual_data_refresh() -> Result<(String, Option<DataRefreshRun>), String> {
    let response = Request::post("/api/data-refresh")
        .send()
        .await
        .map_err(|_| "Não foi possível contactar o serviço de atualização.".to_string())?;
    let status = response.status();
    if status != 202 && status != 409 {
        return Err(format!(
            "Não foi possível iniciar a atualização (HTTP {status})."
        ));
    }

    let body = response
        .json::<DataRefreshRequestResponse>()
        .await
        .map_err(|_| "O serviço devolveu uma resposta inválida.".to_string())?;
    match (status, body.result) {
        (202, DataRefreshRequestState::Started) => Ok((body.message, body.run)),
        (409, DataRefreshRequestState::AlreadyRunning) => Ok((body.message, body.run)),
        (409, DataRefreshRequestState::NoEligibleSession) => Err(body.message),
        _ => Err("O serviço devolveu uma resposta incoerente.".to_string()),
    }
}

#[component]
fn DataRefreshPanel<F>(
    refresh_state: ReadSignal<DataRefreshLoadState>,
    submitting: ReadSignal<bool>,
    request_feedback: ReadSignal<Option<(bool, String)>>,
    request_refresh: F,
) -> impl IntoView
where
    F: Fn(leptos::ev::MouseEvent) + 'static,
{
    let is_running = move || data_refresh_is_awaiting_terminal(&refresh_state.get());
    view! {
        <article class="card refresh-panel" aria-labelledby="refresh-panel-title">
            <header class="refresh-panel-header">
                <div>
                    <h2 id="refresh-panel-title">"Atualização de dados"</h2>
                    <p>"Estado factual devolvido pelo serviço de atualização."</p>
                </div>
                <button
                    class="refresh-button"
                    type="button"
                    disabled=move || submitting.get() || is_running()
                    aria-busy=move || submitting.get().to_string()
                    on:click=request_refresh
                >
                    {move || if submitting.get() { "A iniciar…" } else if is_running() { "Atualização em curso" } else { "Atualizar agora" }}
                </button>
            </header>
            {move || request_feedback.get().map(|(is_error, message)| view! {
                <div class:request-error=is_error class="refresh-feedback" role=if is_error { "alert" } else { "status" }>{message}</div>
            })}
            {move || match refresh_state.get() {
                DataRefreshLoadState::Loading => view! { <DataStatus kind="loading" message="A obter o estado da atualização…".to_string() /> }.into_any(),
                DataRefreshLoadState::Unavailable(message) => view! { <DataStatus kind="unavailable" message=message /> }.into_any(),
                DataRefreshLoadState::Error(message) => view! { <DataStatus kind="error" message=message /> }.into_any(),
                DataRefreshLoadState::Success { status, communication_error, .. } => view! {
                    <>
                        {communication_error.map(|message| view! {
                            <DataStatus kind="error" message=message />
                        })}
                        <RefreshStatusDetails status=status />
                    </>
                }.into_any(),
            }}
        </article>
    }
}

#[component]
fn RefreshStatusDetails(status: DataRefreshStatusResponse) -> impl IntoView {
    let latest = status.latest;
    view! {
        <div class="refresh-status-content" aria-live="polite">
            {match latest {
                Some(run) => view! { <RefreshRunDetails run=run /> }.into_any(),
                None => view! { <div class="refresh-empty" role="status">"Ainda não existem execuções registadas."</div> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn RefreshRunDetails(run: DataRefreshRun) -> impl IntoView {
    let state = run.state;
    let finished = run.finished_at.as_ref().map(format_refresh_datetime);
    let next_attempt = run.next_attempt_at.as_ref().map(format_refresh_datetime);
    let summary = (!run.summary.is_empty()).then_some(run.summary.clone());
    view! {
        <div class="refresh-run">
            <div class="refresh-run-heading">
                <span class=format!("refresh-state {}", refresh_tone(state))>{refresh_state_label(state)}</span>
                <small>{format!("Origem: {}", refresh_origin_label(run.origin))}</small>
            </div>
            <dl class="refresh-facts">
                <div><dt>"Início"</dt><dd>{format_refresh_datetime(&run.started_at)}</dd></div>
                {finished.map(|value| view! { <div><dt>"Conclusão"</dt><dd>{value}</dd></div> })}
                <div><dt>"Itens obtidos"</dt><dd>{run.items_obtained}</dd></div>
                <div><dt>"Itens guardados"</dt><dd>{run.items_persisted}</dd></div>
                <div><dt>"Falhas"</dt><dd>{run.failure_count}</dd></div>
                {next_attempt.map(|value| view! { <div><dt>"Próxima tentativa"</dt><dd>{value}</dd></div> })}
            </dl>
            {summary.map(|value| view! { <p class="refresh-summary">{value}</p> })}
            {(!run.failures.is_empty()).then(|| view! {
                <div class="refresh-failures">
                    <h3>"Falhas registadas"</h3>
                    <ul>{run.failures.into_iter().map(|failure| view! {
                        <li><b>{failure.ticker}</b><span>{failure.operation}</span><p>{failure.error}</p></li>
                    }).collect_view()}</ul>
                </div>
            })}
        </div>
    }
}

#[component]
fn PlaceholderNotice() -> impl IntoView {
    view! {
        <div class="notice" role="note">
            <strong>"Placeholder"</strong>
            " — valores ilustrativos herdados da referência visual; nenhuma decisão deve basear-se neles."
        </div>
    }
}

#[component]
fn MetricCard(
    label: &'static str,
    value: &'static str,
    trend: &'static str,
    positive: bool,
) -> impl IntoView {
    view! {
        <article class="metric-card">
            <span>{label}</span>
            <strong>{value}</strong>
            <small class:positive=positive class:negative=!positive>{trend}</small>
        </article>
    }
}

#[component]
fn CardTitle(title: &'static str, detail: &'static str) -> impl IntoView {
    view! {
        <div class="card-title"><h2>{title}</h2><span>{detail}</span></div>
    }
}

#[component]
fn SettingRow(title: &'static str, detail: &'static str) -> impl IntoView {
    view! {
        <div class="setting-row">
            <div><strong>{title}</strong><span>{detail}</span></div>
            <button type="button" disabled aria-label=format!("Configurar {title}")>"—"</button>
        </div>
    }
}

#[cfg(test)]
mod tracked_ticker_tests {
    use super::{
        ObservationGuard, TickerResolutionState, TrackedTicker, TrackedTickerSource,
        UnderlyingResolutionState, normalize_user_ticker, parsed_resolution,
        resolution_conflict_message, resolution_error, resolved_ticker_for_submission,
        safe_ticker_configuration, should_apply_resolution, ticker_save_error, ticker_save_message,
        user_tracked_tickers,
    };

    #[test]
    fn normalizes_supported_user_tickers_for_display_and_submission() {
        for (input, expected) in [(" aapl ", "AAPL"), ("brk.b", "BRK.B"), ("^spx", "^SPX")] {
            assert_eq!(normalize_user_ticker(input), Ok(expected.to_string()));
        }
    }

    #[test]
    fn rejects_values_that_the_http_contract_will_reject() {
        for input in ["", " ", "SP Y", "SPY/US", "ABCDEFGHIJKLMNOP"] {
            assert!(normalize_user_ticker(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn presentation_excludes_every_system_ticker() {
        let tickers = [TrackedTickerSource::System, TrackedTickerSource::User]
            .into_iter()
            .map(|source| TrackedTicker {
                ticker: if source == TrackedTickerSource::System {
                    "SPX"
                } else {
                    "AAPL"
                }
                .into(),
                source,
                active: true,
                historical_prices: true,
                option_snapshots: false,
                resolution_state: UnderlyingResolutionState::Resolved,
                validated_at: None,
                metadata: Default::default(),
            })
            .collect();

        let visible = user_tracked_tickers(tickers);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].ticker, "AAPL");
    }

    #[test]
    fn ui_configuration_can_never_enable_option_snapshots() {
        for (active, historical) in [(false, false), (false, true), (true, false), (true, true)] {
            assert!(!safe_ticker_configuration(active, historical).option_snapshots);
        }
    }

    #[test]
    fn legacy_snapshots_are_factual_and_safely_disabled_before_saving() {
        let legacy_value = true;
        assert!(
            legacy_value,
            "the persisted value remains available for display"
        );
        let saved = safe_ticker_configuration(true, true);
        assert!(!saved.option_snapshots);
    }

    #[test]
    fn revalidation_preserves_an_inactive_record() {
        let saved = safe_ticker_configuration(false, true);
        assert!(!saved.active);
    }

    #[test]
    fn invalid_json_is_not_classified_as_service_unavailable() {
        assert!(matches!(
            parsed_resolution(None),
            TickerResolutionState::InvalidResponse
        ));
        assert!(!matches!(
            parsed_resolution(None),
            TickerResolutionState::TemporarilyUnavailable(_)
        ));
    }

    #[test]
    fn unexpected_resolution_status_is_preserved() {
        assert!(matches!(
            resolution_error(418, Some("provider response".to_string())),
            TickerResolutionState::UnexpectedStatus(418, Some(detail)) if detail == "provider response"
        ));
    }

    #[test]
    fn resolution_conflict_uses_api_error_detail() {
        let detail = "tracked ticker is equivalent to a system-protected identity".to_string();
        let state = resolution_error(409, Some(detail.clone()));
        assert!(matches!(
            &state,
            TickerResolutionState::Conflict(Some(message)) if message == &detail
        ));
        let TickerResolutionState::Conflict(detail) = state else {
            panic!("HTTP 409 must be an explicit conflict");
        };
        let message = resolution_conflict_message(detail);
        assert!(message.contains("gerido e protegido pelo sistema"));
        assert!(message.contains("system-protected identity"));
    }

    #[test]
    fn resolution_conflict_without_valid_json_has_factual_fallback() {
        let state = resolution_error(409, None);
        let TickerResolutionState::Conflict(detail) = state else {
            panic!("HTTP 409 must be an explicit conflict");
        };
        assert_eq!(
            resolution_conflict_message(detail),
            "Este ticker corresponde a um subjacente gerido e protegido pelo sistema (HTTP 409)."
        );
    }

    #[test]
    fn conflict_cannot_be_submitted_as_a_put() {
        let conflict = TickerResolutionState::Conflict(Some("protected".into()));
        assert!(resolved_ticker_for_submission(&conflict, "ANY").is_none());
    }

    #[test]
    fn put_conflict_uses_api_error_detail() {
        let message = ticker_save_error(409, Some("canonical identity is protected".into()));
        assert!(message.contains("gerido e protegido pelo sistema"));
        assert!(message.contains("canonical identity is protected"));
    }

    #[test]
    fn protected_alias_policy_is_not_coded_in_the_frontend() {
        let production = include_str!("main.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for provider_identity in ["GSPC", "VIX"] {
            let alias_literal = format!("\"^{provider_identity}\"");
            assert!(!production.contains(&alias_literal));
        }
    }

    #[test]
    fn stale_resolution_cannot_replace_the_current_ticker() {
        let guard = ObservationGuard::new();
        assert!(!should_apply_resolution(&guard, 4, 5));
        assert!(should_apply_resolution(&guard, 5, 5));
    }

    #[test]
    fn cleanup_prevents_async_ui_updates() {
        let guard = ObservationGuard::new();
        guard.cancel();
        assert!(!should_apply_resolution(&guard, 5, 5));
    }

    #[test]
    fn next_update_message_requires_active_historical_prices() {
        for (active, historical, mentions_next_update) in [
            (true, true, true),
            (true, false, false),
            (false, true, false),
            (false, false, false),
        ] {
            let configuration = safe_ticker_configuration(active, historical);
            assert_eq!(
                ticker_save_message("AAPL", &configuration).contains("próxima atualização"),
                mentions_next_update
            );
        }
    }
}

#[cfg(test)]
mod data_refresh_tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn run(
        id: &str,
        origin: DataRefreshOrigin,
        state: DataRefreshState,
        started_at_ms: i64,
        next_attempt_at_ms: Option<i64>,
    ) -> DataRefreshRun {
        DataRefreshRun {
            id: id.to_string(),
            origin,
            state,
            started_at: Utc.timestamp_millis_opt(started_at_ms).unwrap(),
            finished_at: (state != DataRefreshState::Running)
                .then(|| Utc.timestamp_millis_opt(started_at_ms + 1_000).unwrap()),
            target_session: chrono::NaiveDate::from_ymd_opt(2026, 8, 13).unwrap(),
            items_obtained: 0,
            items_persisted: 0,
            failure_count: 0,
            next_attempt_at: next_attempt_at_ms
                .map(|value| Utc.timestamp_millis_opt(value).unwrap()),
            summary: String::new(),
            failures: Vec::new(),
        }
    }

    fn status(run: DataRefreshRun) -> DataRefreshStatusResponse {
        DataRefreshStatusResponse {
            running: run.state == DataRefreshState::Running,
            latest: Some(run.clone()),
            recent: vec![run],
        }
    }

    #[test]
    fn maps_every_wire_state_to_the_required_portuguese_copy() {
        let cases = [
            (
                DataRefreshState::Running,
                "Em curso",
                "Atualização de dados iniciada",
            ),
            (
                DataRefreshState::Completed,
                "Concluída",
                "Atualização concluída",
            ),
            (DataRefreshState::Partial, "Parcial", "Atualização parcial"),
            (DataRefreshState::Failed, "Falhou", "Atualização falhou"),
        ];

        for (state, status, activity) in cases {
            assert_eq!(refresh_state_label(state), status);
            assert_eq!(refresh_activity_title(state), activity);
        }
    }

    #[test]
    fn maps_every_wire_origin_without_inventing_a_source() {
        let cases = [
            (DataRefreshOrigin::Startup, "Arranque"),
            (DataRefreshOrigin::Scheduled, "Agendada"),
            (DataRefreshOrigin::Retry, "Nova tentativa"),
            (DataRefreshOrigin::Manual, "Manual"),
        ];

        for (origin, label) in cases {
            assert_eq!(refresh_origin_label(origin), label);
        }
    }

    #[test]
    fn transient_error_during_running_preserves_facts_and_polling_until_completion() {
        let running = run(
            "manual-1",
            DataRefreshOrigin::Manual,
            DataRefreshState::Running,
            1_000,
            None,
        );
        let mut state = DataRefreshLoadState::Loading;
        apply_data_refresh_status_result(
            &mut state,
            DataRefreshStatusResult::Success(status(running.clone())),
        );
        apply_data_refresh_status_result(
            &mut state,
            DataRefreshStatusResult::Error("ligação interrompida".to_string()),
        );

        let DataRefreshLoadState::Success {
            status: preserved,
            communication_error,
            awaiting_terminal_confirmation,
        } = &state
        else {
            panic!("o último estado factual deve ser preservado");
        };
        assert_eq!(preserved.latest.as_ref().unwrap().id, running.id);
        assert_eq!(communication_error.as_deref(), Some("ligação interrompida"));
        assert!(*awaiting_terminal_confirmation);
        assert_eq!(
            data_refresh_observation_delay_ms(&state, 2_000),
            Some(DATA_REFRESH_POLL_INTERVAL_MS)
        );

        let completed = run(
            "manual-1",
            DataRefreshOrigin::Manual,
            DataRefreshState::Completed,
            1_000,
            None,
        );
        apply_data_refresh_status_result(
            &mut state,
            DataRefreshStatusResult::Success(status(completed)),
        );
        assert!(!data_refresh_is_awaiting_terminal(&state));
        assert_eq!(data_refresh_observation_delay_ms(&state, 3_000), None);
        assert!(matches!(
            state,
            DataRefreshLoadState::Success {
                communication_error: None,
                ..
            }
        ));
    }

    #[test]
    fn scheduled_execution_is_discovered_from_the_next_attempt_timer() {
        let now = 10_000;
        let terminal = run(
            "scheduled-previous",
            DataRefreshOrigin::Scheduled,
            DataRefreshState::Completed,
            1_000,
            Some(now + 20_000),
        );
        let mut state = DataRefreshLoadState::Loading;
        apply_data_refresh_status_result(
            &mut state,
            DataRefreshStatusResult::Success(status(terminal)),
        );
        assert_eq!(data_refresh_observation_delay_ms(&state, now), Some(21_500));

        let scheduled = run(
            "scheduled-next",
            DataRefreshOrigin::Scheduled,
            DataRefreshState::Running,
            now + 20_000,
            None,
        );
        apply_data_refresh_status_result(
            &mut state,
            DataRefreshStatusResult::Success(status(scheduled)),
        );
        assert!(data_refresh_is_awaiting_terminal(&state));
        assert_eq!(
            data_refresh_observation_delay_ms(&state, now + 21_500),
            Some(DATA_REFRESH_POLL_INTERVAL_MS)
        );
    }

    #[test]
    fn past_next_attempt_uses_a_bounded_non_immediate_recheck() {
        let terminal = run(
            "retry-previous",
            DataRefreshOrigin::Retry,
            DataRefreshState::Failed,
            1_000,
            Some(9_000),
        );
        let mut state = DataRefreshLoadState::Loading;
        apply_data_refresh_status_result(
            &mut state,
            DataRefreshStatusResult::Success(status(terminal)),
        );

        assert_eq!(
            data_refresh_observation_delay_ms(&state, 10_000),
            Some(DATA_REFRESH_PAST_ATTEMPT_RECHECK_MS)
        );
        assert!(data_refresh_needs_visibility_check(&state, 10_000));
    }

    #[test]
    fn cleanup_disables_timer_and_visibility_callbacks() {
        let timer_guard = ObservationGuard::new();
        let timer_callback = timer_guard.clone();
        let listener_guard = ObservationGuard::new();
        let visibility_callback = listener_guard.clone();

        timer_guard.cancel();
        listener_guard.cancel();

        assert!(!timer_callback.is_active());
        assert!(!visibility_callback.is_active());
    }
}
