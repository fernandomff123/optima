use super::{ShellIcon, ShellIconKind};
use leptos::prelude::*;

#[component]
pub fn Topbar() -> impl IntoView {
    view! {
        <header class="flex h-14 items-center gap-5 border-b border-border bg-canvas px-4">
            <label class="relative min-w-0 max-w-[39rem] flex-1">
                <span class="sr-only">"Search assets"</span>
                <span class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-muted-readable" aria-hidden="true">
                    <ShellIcon kind=ShellIconKind::Search class="size-4" />
                </span>
                <input class="h-9 w-full rounded border border-border bg-surface py-0 pl-9 pr-10 text-sm text-text-secondary placeholder:text-text-muted-readable" type="search" placeholder="Search ticker, company, ETF or index…" readonly aria-disabled="true" />
                <kbd class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded border border-border px-1.5 py-0.5 text-[0.625rem] text-text-muted-readable">"/"</kbd>
            </label>
            <div class="ml-auto flex h-full items-center gap-4">
                <span class="hidden items-center gap-2 text-xs font-semibold text-text-secondary md:flex">
                    <span class="size-1.5 rounded-full bg-finance-positive" aria-hidden="true"></span>"MARKET OPEN"
                </span>
                <time class="numeric hidden text-xs text-text-secondary sm:block" datetime="2026-08-28T09:45:31-04:00">"09:45:31 ET"</time>
                <span class="h-5 border-l border-border" aria-hidden="true"></span>
                <DisabledAction label="Notifications"><ShellIcon kind=ShellIconKind::Bell class="size-5" /></DisabledAction>
                <DisabledAction label="Settings"><ShellIcon kind=ShellIconKind::Settings class="size-5" /></DisabledAction>
                <DisabledAction label="App launcher"><ShellIcon kind=ShellIconKind::Launcher class="size-5" /></DisabledAction>
            </div>
        </header>
    }
}

#[component]
fn DisabledAction(#[prop(into)] label: &'static str, children: Children) -> impl IntoView {
    view! {
        <button class="grid size-8 cursor-not-allowed place-items-center text-text-secondary opacity-70" type="button" aria-label=format!("{label} unavailable") title=format!("{label} unavailable") disabled>{children()}</button>
    }
}
