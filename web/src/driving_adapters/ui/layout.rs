use super::components::{MobileNavigation, Sidebar, Topbar};
use leptos::prelude::*;

#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    view! {
        <a class="fixed left-3 top-3 z-50 -translate-y-20 rounded bg-interactive-source px-4 py-2 text-sm font-semibold text-white focus:translate-y-0" href="#main-content">"Skip to content"</a>
        <div class="min-h-screen bg-canvas text-text-primary lg:flex">
            <Sidebar />
            <div class="min-w-0 flex-1 overflow-x-hidden">
                <Topbar /><MobileNavigation />
                <main id="main-content" class="min-h-[calc(100vh-3.5rem)]" tabindex="-1">{children()}</main>
            </div>
        </div>
    }
}
