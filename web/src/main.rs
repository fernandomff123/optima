use api_models::{
    AssetLivePrice, DataState, Freshness, MarketSpxHistoryResponse, PriceHistoryOverview,
    PriceHistoryPoint,
};
use futures_util::StreamExt;
use gloo_net::{http::Request, websocket::futures::WebSocket};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::fmt;

const API_BASE_PATH: &str = "/api";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Sectors,
    Portfolio,
    Builder,
    MarketAnalysis,
    Simulator,
    Settings,
}

impl Page {
    const ALL: [Self; 7] = [
        Self::Dashboard,
        Self::Sectors,
        Self::Portfolio,
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
            Self::Builder => "⌘",
            Self::MarketAnalysis => "⌁",
            Self::Simulator => "◎",
            Self::Settings => "⚙",
        }
    }

    const fn eyebrow(self) -> &'static str {
        match self {
            Self::Dashboard => "Visão geral",
            Self::Sectors | Self::MarketAnalysis => "Mercado",
            Self::Portfolio => "Património",
            Self::Builder => "Estratégias",
            Self::Simulator => "Cenários",
            Self::Settings => "Preferências",
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
                    Page::Sectors => view! {
                        <ShellPlaceholderPage
                            eyebrow=Page::Sectors.eyebrow()
                            title="Setores"
                            subtitle="Mapa de calor e força relativa dos setores do S&P 500"
                        />
                    }.into_any(),
                    Page::Portfolio => view! { <PortfolioPage /> }.into_any(),
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

#[component]
fn ShellPlaceholderPage(
    eyebrow: &'static str,
    title: &'static str,
    subtitle: &'static str,
) -> impl IntoView {
    view! {
        <section class="page placeholder-page">
            <PageHeader eyebrow=eyebrow title=title subtitle=subtitle />
            <div class="empty-page-state" role="status">
                <span class="empty-page-icon" aria-hidden="true">"◇"</span>
                <strong>"Página preparada no novo shell"</strong>
                <p>"Conteúdo ainda não implementado. Não existem dados financeiros associados a esta vista."</p>
            </div>
        </section>
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
                "Simulação · todos os valores, sinais e atividades desta página são demonstrativos."
            </p>

            <div class="dashboard-metrics">
                {data.metrics.into_iter().map(|metric| view! { <DemoMetric metric=metric /> }).collect_view()}
            </div>

            <div class="official-dashboard-grid">
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

                <DashboardPanel class="pnl-panel" title="P&L vs benchmark" subtitle="Evolução acumulada · últimos 30 dias · simulação">
                    <DemoPnlChart data=data.pnl />
                </DashboardPanel>

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

                <DashboardPanel class="maturity-panel" title="Escada de vencimentos" subtitle="Posições por intervalo de DTE · simulação">
                    <div class="ladder-bars" role="img" aria-label="Distribuição demonstrativa: 3 posições de 0 a 7 DTE, 2 de 8 a 14, 4 de 15 a 30, 2 de 31 a 60 e 1 acima de 60">
                        {data.maturities.into_iter().map(|item| view! {
                            <div class="ladder-column"><b>{item.count}</b><div><i class=item.tone style=format!("height:{}px", item.height)></i></div><span>{item.range}</span></div>
                        }).collect_view()}
                    </div>
                    <small class="ladder-axis">"Dias até ao vencimento (DTE)"</small>
                    <div class="ladder-note"><b>"5 posições"</b><span>"vencem nos próximos 14 dias"</span></div>
                </DashboardPanel>

                <DashboardPanel class="activity-panel" title="Atividade recente" subtitle="Registo demonstrativo; nenhuma ação real">
                    <div class="activity-list">
                        {data.activities.into_iter().map(|activity| view! {
                            <div class="activity-row">
                                <span class=format!("activity-icon {}", activity.tone) aria-hidden="true">{activity.icon}</span>
                                <div><small>{activity.time}</small><b>{activity.title}</b><p>{activity.detail}</p></div>
                                {activity.value.map(|value| view! { <strong class="positive">{value}</strong> })}
                            </div>
                        }).collect_view()}
                    </div>
                </DashboardPanel>

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
    const WIDTH: f64 = 900.0;
    const HEIGHT: f64 = 210.0;
    const PAD_X: f64 = 22.0;
    const PAD_TOP: f64 = 22.0;
    const PAD_BOTTOM: f64 = 50.0;

    let min = points
        .iter()
        .map(|point| point.close)
        .fold(f64::INFINITY, f64::min);
    let max = points
        .iter()
        .map(|point| point.close)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1.0);
    let denominator = points.len().saturating_sub(1).max(1) as f64;
    let polyline = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let x = PAD_X + index as f64 / denominator * (WIDTH - PAD_X * 2.0);
            let y = PAD_TOP + (max - point.close) / range * (HEIGHT - PAD_TOP - PAD_BOTTOM);
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let first = points.first().map(|point| point.date);
    let last = points.last().map(|point| point.date);
    let first_label = first.map(|date| date.to_string()).unwrap_or_default();
    let last_label = last.map(|date| date.to_string()).unwrap_or_default();
    let description = format!(
        "{} sessões, de {first_label} a {last_label}, entre {min:.2} e {max:.2} pontos",
        points.len()
    );

    view! {
        <figure class="spx-chart">
            <svg viewBox=format!("0 0 {WIDTH} {HEIGHT}") role="img" aria-labelledby="spx-chart-title spx-chart-desc">
                <title id="spx-chart-title">"Histórico de fechos do S&P 500"</title>
                <desc id="spx-chart-desc">{description}</desc>
                <g class="chart-grid" aria-hidden="true">
                    <line x1="22" y1="22" x2="878" y2="22" />
                    <line x1="22" y1="91" x2="878" y2="91" />
                    <line x1="22" y1="160" x2="878" y2="160" />
                </g>
                <polyline class="spx-line" points=polyline />
                <text x="22" y="16" class="chart-value">{format_number(max)}</text>
                <text x="22" y="174" class="chart-value">{format_number(min)}</text>
                <text x="22" y="200" class="chart-date">{first_label.clone()}</text>
                <text x="878" y="200" text-anchor="end" class="chart-date">{last_label.clone()}</text>
                <text x="450" y="200" text-anchor="middle" class="chart-session-count">{format!("{} sessões", points.len())}</text>
            </svg>
        </figure>
    }
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
    view! {
        <section class="page">
            <PageHeader eyebrow="Preferências" title="Configurações" subtitle="Preferências locais da interface" />
            <PlaceholderNotice />
            <article class="card settings-list">
                <SettingRow title="Modo escuro" detail="Tema inicial da fundação visual" />
                <SettingRow title="Alertas de risco" detail="Configuração placeholder" />
                <SettingRow title="Moeda base" detail="EUR (€) · configuração placeholder" />
            </article>
        </section>
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
