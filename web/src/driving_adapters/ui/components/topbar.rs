use super::{ShellIcon, ShellIconKind};
use leptos::prelude::*;

#[component]
pub fn Topbar() -> impl IntoView {
    view! {
        <header class="flex h-14 items-center justify-between gap-4 border-b border-border bg-canvas px-4 sm:px-5">
            <div class="flex min-w-0 items-center gap-3">
                <span class="grid size-8 shrink-0 place-items-center rounded bg-interactive-source text-sm font-black text-white lg:hidden">"O"</span>
                <div class="min-w-0">
                    <p class="truncate text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-text-muted-readable">"Market workspace"</p>
                    <p class="truncate text-xs font-semibold text-text-primary">"Optima Terminal"</p>
                </div>
            </div>
            <label class="relative hidden min-w-0 max-w-[39rem] flex-1 lg:block">
                <span class="sr-only">"Search assets"</span>
                <span class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-muted-readable" aria-hidden="true">
                    <ShellIcon kind=ShellIconKind::Search class="size-4" />
                </span>
                <input class="h-9 w-full rounded border border-border bg-surface py-0 pl-9 pr-3 text-sm text-text-secondary placeholder:text-text-muted-readable disabled:cursor-not-allowed disabled:opacity-80" type="search" placeholder="Search ticker, company, ETF or index…" disabled />
            </label>
            <div class="flex items-center gap-2">
                <span class="hidden items-center gap-2 rounded border border-border bg-surface px-3 py-2 text-xs text-text-secondary sm:flex">
                    <span class="size-1.5 rounded-full bg-level-special" aria-hidden="true"></span>"Foundation mode"
                </span>
                <button class="rounded border border-border bg-surface-elevated px-3 py-2 text-xs font-semibold text-text-secondary disabled:cursor-not-allowed disabled:opacity-50" type="button" aria-label="Open workspace commands" disabled>"⌘ K"</button>
            </div>
        </header>
    }
}
