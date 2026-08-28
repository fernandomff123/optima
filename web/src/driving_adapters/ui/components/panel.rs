use leptos::prelude::*;

#[component]
pub fn Panel(
    #[prop(into)] title: Signal<String>,
    #[prop(optional, into)] eyebrow: Option<String>,
    #[prop(optional, into)] badge: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <section class="rounded-panel border border-border bg-surface shadow-panel">
            <header class="flex min-h-14 items-center justify-between border-b border-border px-4 sm:px-5">
                <div>
                    {eyebrow.map(|value| view! { <p class="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-text-muted-readable">{value}</p> })}
                    <h2 class="text-sm font-semibold tracking-tight text-text-primary">{title}</h2>
                </div>
                {badge.map(|value| view! { <span class="rounded border border-border bg-surface-elevated px-2 py-1 text-[0.625rem] font-medium uppercase tracking-wider text-text-muted-readable">{value}</span> })}
            </header>
            <div class="p-4 sm:p-5">{children()}</div>
        </section>
    }
}
