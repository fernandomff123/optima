use super::pages::{
    AssetChartPage, AssetGexPage, AssetOptionsPage, AssetOverviewPage, AssetRedirect,
    AssetSimulationPage, AssetVolatilityPage, AssetsPage, DashboardPage, MarketsPage, NotFoundPage,
    PortfolioPage, SettingsPage,
};
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Routes},
    path,
};

#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=NotFoundPage>
            <Route path=path!("") view=DashboardPage />
            <Route path=path!("markets") view=MarketsPage />
            <Route path=path!("assets") view=AssetsPage />
            <Route path=path!("assets/:ticker") view=AssetRedirect />
            <Route path=path!("assets/:ticker/overview") view=AssetOverviewPage />
            <Route path=path!("assets/:ticker/chart") view=AssetChartPage />
            <Route path=path!("assets/:ticker/options") view=AssetOptionsPage />
            <Route path=path!("assets/:ticker/volatility") view=AssetVolatilityPage />
            <Route path=path!("assets/:ticker/gex") view=AssetGexPage />
            <Route path=path!("assets/:ticker/simulation") view=AssetSimulationPage />
            <Route path=path!("portfolio") view=PortfolioPage />
            <Route path=path!("settings") view=SettingsPage />
        </Routes>
    }
}
