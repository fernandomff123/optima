use leptos::prelude::*;
use leptos_router::components::Router;
use std::rc::Rc;

use crate::{
    application::asset_chart::AssetChartUseCase,
    application::asset_options::AssetOptionsUseCase,
    application::asset_overview::AssetOverviewUseCase,
    application::asset_simulation::AssetSimulationUseCase,
    application::asset_volatility::AssetVolatilityUseCase,
    application::technical_indicators::TechnicalIndicatorsUseCase,
    driven_adapters::mocks::{
        asset_chart::MockAssetChartAdapter, asset_options::MockAssetOptionsAdapter,
        asset_overview::MockAssetOverviewAdapter, asset_simulation::MockAssetSimulationAdapter,
        asset_volatility::MockAssetVolatilityAdapter,
    },
    driven_adapters::technical_indicators::yata::YataTechnicalIndicatorAdapter,
    driving_adapters::ui::{layout::AppShell, router::AppRoutes},
};

pub fn asset_overview_use_case() -> AssetOverviewUseCase {
    AssetOverviewUseCase::new(Rc::new(MockAssetOverviewAdapter))
}

pub fn asset_options_use_case() -> AssetOptionsUseCase {
    AssetOptionsUseCase::new(Rc::new(MockAssetOptionsAdapter))
}

pub fn asset_chart_use_case() -> AssetChartUseCase {
    AssetChartUseCase::new(
        Rc::new(MockAssetChartAdapter),
        TechnicalIndicatorsUseCase::new(Rc::new(YataTechnicalIndicatorAdapter)),
    )
}

pub fn asset_simulation_use_case() -> AssetSimulationUseCase {
    AssetSimulationUseCase::new(Rc::new(MockAssetSimulationAdapter))
}

pub fn asset_volatility_use_case() -> AssetVolatilityUseCase {
    AssetVolatilityUseCase::new(Rc::new(MockAssetVolatilityAdapter))
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <AppShell><AppRoutes /></AppShell>
        </Router>
    }
}
