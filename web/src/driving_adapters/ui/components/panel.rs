use leptos::prelude::*;

#[component]
pub fn Panel(
    #[prop(into)] title: Signal<String>,
    #[prop(optional, into)] eyebrow: Option<String>,
    #[prop(optional, into)] badge: Option<String>,
    #[prop(optional)] compact: bool,
    children: Children,
) -> impl IntoView {
    let header_class = if compact {
        "panel-header"
    } else {
        "flex min-h-14 items-center justify-between border-b border-border px-4 sm:px-5"
    };
    let body_class = if compact {
        "flex min-h-0 min-w-0 flex-1 flex-col p-0"
    } else {
        "p-4 sm:p-5"
    };
    let section_class = if compact {
        "flex h-full min-w-0 flex-col border border-border bg-surface"
    } else {
        "rounded-panel border border-border bg-surface shadow-panel"
    };
    view! {
        <section class=section_class>
            <header class=header_class>
                <div>
                    {eyebrow.map(|value| view! { <p class="mb-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-text-muted-readable">{value}</p> })}
                    <h2 class=if compact { "text-base font-semibold tracking-tight text-text-primary" } else { "text-sm font-semibold tracking-tight text-text-primary" }>{title}</h2>
                </div>
                {badge.map(|value| view! { <span class="rounded border border-border bg-surface-elevated px-2 py-1 text-[0.625rem] font-medium uppercase tracking-wider text-text-muted-readable">{value}</span> })}
            </header>
            <div class=body_class>{children()}</div>
        </section>
    }
}
