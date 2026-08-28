use crate::{application::read_models::FeedbackState, driving_adapters::ui::components::DataState};
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="mx-auto max-w-xl p-6 pt-20 text-center">
            <p class="mb-2 text-xs font-semibold uppercase tracking-[0.2em] text-interactive-text">"404"</p>
            <h1 class="mb-5 text-2xl font-semibold text-text-primary">"Route not found"</h1>
            <DataState state=FeedbackState::terminal_error() />
            <A href="/" attr:class="mt-5 inline-flex rounded bg-interactive-source px-4 py-2 text-sm font-semibold text-white hover:bg-state-focus">"Return to dashboard"</A>
        </div>
    }
}
