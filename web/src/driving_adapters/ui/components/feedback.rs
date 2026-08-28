use crate::application::read_models::{FeedbackKind, FeedbackState};
use leptos::prelude::*;

#[component]
pub fn DataState(state: FeedbackState) -> impl IntoView {
    let (marker, marker_class, role) = match state.kind {
        FeedbackKind::Loading => (
            "…",
            "border-interactive-source text-interactive-text",
            "status",
        ),
        FeedbackKind::Empty => (
            "○",
            "border-text-muted-readable text-text-muted-readable",
            "status",
        ),
        FeedbackKind::Stale => ("!", "border-level-special text-level-special", "status"),
        FeedbackKind::Unavailable => (
            "—",
            "border-text-muted-readable text-text-muted-readable",
            "status",
        ),
        FeedbackKind::RecoverableError => {
            ("↻", "border-finance-negative text-negative-text", "alert")
        }
        FeedbackKind::TerminalError => ("×", "border-finance-negative text-negative-text", "alert"),
    };
    view! {
        <div class="flex min-h-36 items-center justify-center rounded border border-dashed border-border bg-canvas/40 p-5 text-center" role=role>
            <div class="max-w-sm">
                <span class=format!("mx-auto mb-3 grid size-8 place-items-center rounded-full border text-sm font-bold {marker_class}") aria-hidden="true">{marker}</span>
                <p class="text-sm font-semibold text-text-primary">{state.title}</p>
                <p class="mt-1 text-xs leading-5 text-text-muted-readable">{state.detail}</p>
            </div>
        </div>
    }
}
