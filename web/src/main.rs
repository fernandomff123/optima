use api_models::{
    AssetLivePrice, DataState, Freshness, MarketSpxHistoryResponse, PriceHistoryOverview,
    PriceHistoryPoint,
};
use futures_util::StreamExt;
use gloo_net::{http::Request, websocket::futures::WebSocket};
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
                    Page::MarketAnalysis => view! {
                        <ShellPlaceholderPage
                            eyebrow=Page::MarketAnalysis.eyebrow()
                            title="Análise de mercado"
                            subtitle="Visão agregada de índices, volatilidade e condições de mercado"
                        />
                    }.into_any(),
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

#[component]
fn DashboardPage() -> impl IntoView {
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
            let socket = match WebSocket::open(&socket_url) {
                Ok(socket) => socket,
                Err(_) => {
                    set_live_price.set(Some(Err(
                        "Não foi possível ligar ao canal de cotações.".to_string()
                    )));
                    return;
                }
            };
            let (_, mut messages) = socket.split();
            while let Some(message) = messages.next().await {
                match message {
                    Ok(gloo_net::websocket::Message::Text(payload)) => {
                        match serde_json::from_str::<AssetLivePrice>(&payload) {
                            Ok(price) => set_live_price.set(Some(Ok(price))),
                            Err(_) => set_live_price.set(Some(Err(
                                "A cotação recebida não tem o formato esperado.".to_string(),
                            ))),
                        }
                    }
                    Ok(gloo_net::websocket::Message::Bytes(_)) => {}
                    Err(_) => {
                        set_live_price.set(Some(Err(
                            "A ligação ao canal de cotações foi interrompida.".to_string(),
                        )));
                        break;
                    }
                }
            }
        });
    });

    view! {
        <section class="page">
            <PageHeader
                title="Dashboard"
                subtitle="Cotação corrente e últimas 90 sessões disponíveis do S&P 500"
            />
            <div class="metric-grid">
                <LiveSpxCard price=live_price />
                <MetricCard label="VIX · placeholder" value="14,71" trend="Não ligado" positive=true />
                <MetricCard label="Taxa 10Y · placeholder" value="4,38%" trend="Não ligado" positive=false />
                <MetricCard label="Carteira · placeholder" value="€ 24 680" trend="Não ligado" positive=true />
            </div>
            <div class="content-grid wide-left">
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
                <article class="card">
                    <CardTitle title="Volatilidade" detail="Placeholder · ainda não ligado" />
                    <div class="term-list">
                        <TermRow label="VIX" value="14,71" />
                        <TermRow label="VIX3M" value="16,08" />
                        <TermRow label="VIX6M" value="17,42" />
                        <TermRow label="VVIX" value="86,30" />
                    </div>
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
fn TermRow(label: &'static str, value: &'static str) -> impl IntoView {
    view! { <div class="term-row"><span>{label}</span><strong>{value}</strong></div> }
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
