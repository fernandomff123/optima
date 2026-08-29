use leptos::prelude::*;
use leptos_router::components::Router;
use std::rc::Rc;

use crate::{
    application::asset_options::AssetOptionsUseCase,
    application::asset_overview::AssetOverviewUseCase,
    driven_adapters::mocks::{
        asset_options::MockAssetOptionsAdapter, asset_overview::MockAssetOverviewAdapter,
    },
    driving_adapters::ui::{layout::AppShell, router::AppRoutes},
};

pub fn asset_overview_use_case() -> AssetOverviewUseCase {
    AssetOverviewUseCase::new(Rc::new(MockAssetOverviewAdapter))
}

pub fn asset_options_use_case() -> AssetOptionsUseCase {
    AssetOptionsUseCase::new(Rc::new(MockAssetOptionsAdapter))
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <AppShell><AppRoutes /></AppShell>
        </Router>
    }
}
