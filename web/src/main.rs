use api_models::ViewContext;
use leptos::prelude::*;

const API_BASE_PATH: &str = "/api";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Portfolio,
    Builder,
    Simulator,
    Settings,
}

impl Page {
    const ALL: [Self; 5] = [
        Self::Dashboard,
        Self::Portfolio,
        Self::Builder,
        Self::Simulator,
        Self::Settings,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Portfolio => "Portfolio",
            Self::Builder => "Construtor",
            Self::Simulator => "Simulador",
            Self::Settings => "Configurações",
        }
    }

    const fn icon(self) -> &'static str {
        match self {
            Self::Dashboard => "▦",
            Self::Portfolio => "◫",
            Self::Builder => "＋",
            Self::Simulator => "⌁",
            Self::Settings => "⚙",
        }
    }

    const fn view_context(self) -> ViewContext {
        match self {
            Self::Dashboard => ViewContext::Market,
            Self::Portfolio => ViewContext::Portfolio,
            Self::Builder => ViewContext::Options,
            Self::Simulator => ViewContext::Simulation,
            Self::Settings => ViewContext::Market,
        }
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (active_page, set_active_page) = signal(Page::Dashboard);

    view! {
        <div class="app-shell">
            <aside class="sidebar">
                <a class="brand" href="#" aria-label="Optima — início">
                    <span class="brand-mark">"O"</span>
                    <span>"Optima"</span>
                </a>

                <nav aria-label="Navegação principal">
                    {Page::ALL.map(|page| {
                        view! {
                            <button
                                type="button"
                                aria-label=page.label()
                                class:active=move || active_page.get() == page
                                aria-current=move || {
                                    (active_page.get() == page).then_some("page")
                                }
                                on:click=move |_| set_active_page.set(page)
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
                        <strong>"Frontend local"</strong>
                        <small>{format!("API via {API_BASE_PATH}")}</small>
                    </div>
                </div>
            </aside>

            <main>
                <div class="topbar">
                    <span class="eyebrow">"OPTIMA · FUNDAÇÃO LEPTOS"</span>
                    <span class="placeholder-pill">"Dados de demonstração"</span>
                </div>
                {move || match active_page.get() {
                    Page::Dashboard => view! { <DashboardPage /> }.into_any(),
                    Page::Portfolio => view! { <PortfolioPage /> }.into_any(),
                    Page::Builder => view! { <BuilderPage /> }.into_any(),
                    Page::Simulator => view! { <SimulatorPage /> }.into_any(),
                    Page::Settings => view! { <SettingsPage /> }.into_any(),
                }}
                <footer>
                    {move || format!(
                        "Contexto público: {:?} · conteúdo placeholder, sem ligação à API",
                        active_page.get().view_context()
                    )}
                </footer>
            </main>
        </div>
    }
}

#[component]
fn PageHeader(title: &'static str, subtitle: &'static str) -> impl IntoView {
    view! {
        <header class="page-header">
            <div>
                <h1>{title}</h1>
                <p>{subtitle}</p>
            </div>
            <button class="secondary-button" type="button" disabled>
                "Atualizar dados"
            </button>
        </header>
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <section class="page">
            <PageHeader
                title="Dashboard"
                subtitle="Visão geral dos mercados, volatilidade e carteira"
            />
            <PlaceholderNotice />
            <div class="metric-grid">
                <MetricCard label="S&P 500" value="5 240,03" trend="+0,42%" positive=true />
                <MetricCard label="VIX" value="14,71" trend="-2,13%" positive=true />
                <MetricCard label="Taxa 10Y" value="4,38%" trend="+0,03" positive=false />
                <MetricCard label="Valor da carteira" value="€ 24 680" trend="+1,18%" positive=true />
            </div>
            <div class="content-grid wide-left">
                <article class="card chart-card">
                    <CardTitle title="Mercado" detail="S&P 500 · 30 dias" />
                    <div class="chart-placeholder" aria-label="Placeholder para gráfico de mercado">
                        <span>"Visualização de série temporal"</span>
                        <div class="fake-line"></div>
                    </div>
                </article>
                <article class="card">
                    <CardTitle title="Volatilidade" detail="Estrutura temporal" />
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

#[component]
fn PortfolioPage() -> impl IntoView {
    view! {
        <section class="page">
            <PageHeader title="Portfolio" subtitle="Posições, exposição e movimentos" />
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
            <PageHeader title="Construtor" subtitle="Composição visual de estratégias de opções" />
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
            <PageHeader title="Simulador" subtitle="Cenários e sensibilidade de opções" />
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
            <PageHeader title="Configurações" subtitle="Preferências locais da interface" />
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
