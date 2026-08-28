use leptos::prelude::*;
use leptos_router::components::Router;

use crate::driving_adapters::ui::{layout::AppShell, router::AppRoutes};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <AppShell><AppRoutes /></AppShell>
        </Router>
    }
}
