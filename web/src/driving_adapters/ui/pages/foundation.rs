use crate::{
    application::read_models::FeedbackState,
    driving_adapters::ui::components::{DataState, Panel},
};
use leptos::prelude::*;

#[component]
fn FoundationPage(
    title: &'static str,
    description: &'static str,
    state: FeedbackState,
) -> impl IntoView {
    view! {
        <div class="p-4 sm:p-6 lg:p-8">
            <header class="mb-6 flex flex-col justify-between gap-3 border-b border-border pb-5 sm:flex-row sm:items-end">
                <div>
                    <p class="mb-2 text-[0.625rem] font-semibold uppercase tracking-[0.18em] text-interactive-text">"Optima / Foundation"</p>
                    <h1 class="text-xl font-semibold tracking-tight text-text-primary sm:text-2xl">{title}</h1>
                    <p class="mt-2 max-w-2xl text-sm leading-6 text-text-secondary">{description}</p>
                </div>
                <span class="w-fit rounded border border-border bg-surface px-3 py-2 text-xs text-text-muted-readable">"No live data"</span>
            </header>
            <div class="grid gap-4 xl:grid-cols-[minmax(0,1.7fr)_minmax(18rem,0.7fr)]">
                <Panel title="Workspace foundation" eyebrow="Structure"><DataState state=state /></Panel>
                <Panel title="Milestone scope" eyebrow="Status">
                    <ul class="space-y-3 text-xs leading-5 text-text-secondary">
                        <li class="flex gap-2"><span class="text-finance-positive" aria-hidden="true">"✓"</span><span>"Leptos CSR and real routing"</span></li>
                        <li class="flex gap-2"><span class="text-finance-positive" aria-hidden="true">"✓"</span><span>"Tailwind semantic tokens"</span></li>
                        <li class="flex gap-2"><span class="text-finance-positive" aria-hidden="true">"✓"</span><span>"Route-scoped Plotly host"</span></li>
                        <li class="flex gap-2"><span class="text-level-special" aria-hidden="true">"○"</span><span>"Financial data intentionally absent"</span></li>
                    </ul>
                </Panel>
            </div>
        </div>
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! { <FoundationPage title="Dashboard" description="The structural home for the future market overview." state=FeedbackState::loading() /> }
}
#[component]
pub fn MarketsPage() -> impl IntoView {
    view! { <FoundationPage title="Markets" description="Market navigation is ready; market contracts are intentionally not connected." state=FeedbackState::stale() /> }
}
#[component]
pub fn AssetsPage() -> impl IntoView {
    view! { <FoundationPage title="Assets" description="Open /assets/SPX to verify route-based asset navigation." state=FeedbackState::empty() /> }
}
#[component]
pub fn OptionsPage() -> impl IntoView {
    view! { <FoundationPage title="Options" description="The global options workspace is structural and contains no real market data." state=FeedbackState::unavailable() /> }
}
#[component]
pub fn VolatilityPage() -> impl IntoView {
    view! { <FoundationPage title="Volatility" description="The global volatility workspace is structural and contains no real market data." state=FeedbackState::unavailable() /> }
}
#[component]
pub fn GexPage() -> impl IntoView {
    view! { <FoundationPage title="GEX / Flow" description="The global flow workspace is structural and contains no real market data." state=FeedbackState::unavailable() /> }
}
#[component]
pub fn SimulationsPage() -> impl IntoView {
    view! { <FoundationPage title="Simulations" description="The global simulations workspace is structural and contains no real market data." state=FeedbackState::unavailable() /> }
}
#[component]
pub fn PortfolioPage() -> impl IntoView {
    view! { <FoundationPage title="Portfolio" description="Portfolio remains structural until its backend contract is approved." state=FeedbackState::unavailable() /> }
}
#[component]
pub fn SettingsPage() -> impl IntoView {
    view! { <FoundationPage title="Settings" description="Local preferences require an authorized use case." state=FeedbackState::recoverable_error() /> }
}
